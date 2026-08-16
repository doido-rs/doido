(() => {
  const { conversationId, userId } = window.CHAT_CONFIG;
  const listEl = document.getElementById("messages-list");
  const loadingEl = document.getElementById("messages-loading");
  const titleEl = document.getElementById("conversation-title");
  const form = document.getElementById("message-form");
  const input = document.getElementById("message-input");
  const attachBtn = document.getElementById("attach-btn");
  const attachmentInput = document.getElementById("attachment-input");
  const statusEl = document.getElementById("composer-status");

  const renderedIds = new Set();
  let cable = null;

  function setStatus(text) {
    if (!text) {
      statusEl.hidden = true;
      return;
    }
    statusEl.textContent = text;
    statusEl.hidden = false;
  }

  function messageBody(msg) {
    if (msg.message_type === "image" && msg.attachment) {
      const caption = msg.body
        ? `<div>${Chat.escapeHtml(msg.body)}</div>`
        : "";
      return `${caption}<img src="${msg.attachment.url}" alt="${Chat.escapeHtml(msg.attachment.filename)}" loading="lazy">`;
    }
    if (msg.message_type === "file" && msg.attachment) {
      const caption = msg.body
        ? `<div>${Chat.escapeHtml(msg.body)}</div>`
        : "";
      return `${caption}<a class="file-link" href="${msg.attachment.url}" target="_blank" rel="noopener">📄 ${Chat.escapeHtml(msg.attachment.filename)}</a>`;
    }
    return Chat.escapeHtml(msg.body || "");
  }

  function appendMessage(msg, scroll = true) {
    if (renderedIds.has(msg.id)) return;
    renderedIds.add(msg.id);

    const li = document.createElement("li");
    li.className = `message ${msg.user_id === userId ? "mine" : "theirs"}`;
    li.dataset.id = msg.id;
    li.innerHTML = `
      ${messageBody(msg)}
      <div class="message-meta">${Chat.formatTime(msg.created_at)}</div>
    `;
    listEl.appendChild(li);

    if (scroll) {
      listEl.scrollTop = listEl.scrollHeight;
    }
  }

  function renderMessages(messages) {
    loadingEl.hidden = true;
    listEl.hidden = false;
    messages.forEach((m) => appendMessage(m, false));
    listEl.scrollTop = listEl.scrollHeight;
  }

  function handleIncomingPayload(payload) {
    if (
      payload &&
      (payload.action === "new_message" || payload.action === "message_sent") &&
      payload.message
    ) {
      appendMessage(payload.message);
    }
  }

  async function loadConversation() {
    const conversation = await Chat.request(`/conversations/${conversationId}`);
    const other = conversation.participants.find((p) => p.id !== userId);
    titleEl.textContent = other ? other.email : `Conversa #${conversationId}`;
  }

  async function loadMessages() {
    try {
      const messages = await Chat.request(
        `/conversations/${conversationId}/messages`
      );
      renderMessages(messages);
    } catch (err) {
      loadingEl.textContent = err.message || "Erro ao carregar mensagens";
    }
  }

  function subscribeCable() {
    const identifier = JSON.stringify({
      channel: "ConversationChannel",
      conversation_id: String(conversationId),
      user_id: String(userId),
    });

    cable = new WebSocket(Chat.cableUrl());

    cable.addEventListener("open", () => {
      cable.send(
        JSON.stringify({ command: "subscribe", identifier })
      );
      setStatus("Conectado");
      setTimeout(() => setStatus(""), 2000);
    });

    cable.addEventListener("message", (event) => {
      try {
        const frame = JSON.parse(event.data);
        if (
          frame.type === "ping" ||
          frame.type === "welcome" ||
          frame.type === "confirm_subscription" ||
          frame.type === "reject_subscription"
        ) {
          return;
        }

        const payload =
          typeof frame.message === "string"
            ? JSON.parse(frame.message)
            : frame.message;

        handleIncomingPayload(payload);
      } catch {
        /* ignore malformed frames */
      }
    });

    cable.addEventListener("close", () => {
      setStatus("Desconectado — reconectando…");
      setTimeout(subscribeCable, 3000);
    });
  }

  function sendText(body) {
    if (!cable || cable.readyState !== WebSocket.OPEN) {
      throw new Error("WebSocket não conectado");
    }
    const identifier = JSON.stringify({
      channel: "ConversationChannel",
      conversation_id: String(conversationId),
      user_id: String(userId),
    });
    cable.send(
      JSON.stringify({
        command: "message",
        identifier,
        data: JSON.stringify({ action: "speak", body }),
      })
    );
  }

  function fileToBase64(file) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result);
      reader.onerror = () => reject(new Error("Falha ao ler o arquivo"));
      reader.readAsDataURL(file);
    });
  }

  async function uploadAttachment(file) {
    const isImage = file.type.startsWith("image/");
    const messageType = isImage ? "image" : "file";

    setStatus("Enviando arquivo…");

    if (isImage) {
      const imageData = await fileToBase64(file);
      const message = await Chat.request("/messages", {
        method: "POST",
        body: JSON.stringify({
          conversation_id: conversationId,
          message_type: messageType,
          image_data: imageData,
          image_content_type: file.type || "application/octet-stream",
          image_filename: file.name,
        }),
      });
      appendMessage(message);
      setStatus("");
      return;
    }

    const meta = await Chat.request("/doido/storage/direct_uploads", {
      method: "POST",
      body: JSON.stringify({
        filename: file.name,
        content_type: file.type || "application/octet-stream",
        byte_size: file.size,
      }),
    });

    const upload = meta.direct_upload;
    const putRes = await fetch(upload.url, {
      method: "PUT",
      headers: upload.headers,
      body: file,
    });
    if (!putRes.ok) throw new Error("Falha no upload do arquivo");

    const message = await Chat.request("/messages", {
      method: "POST",
      body: JSON.stringify({
        conversation_id: conversationId,
        message_type: messageType,
        attachment_signed_id: meta.signed_id,
      }),
    });

    appendMessage(message);
    setStatus("");
  }

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const body = input.value.trim();
    if (!body) return;

    input.value = "";
    try {
      sendText(body);
    } catch (err) {
      input.value = body;
      setStatus(err.message || "Erro ao enviar");
    }
  });

  attachBtn.addEventListener("click", () => attachmentInput.click());

  attachmentInput.addEventListener("change", async () => {
    const file = attachmentInput.files[0];
    attachmentInput.value = "";
    if (!file) return;

    try {
      await uploadAttachment(file);
    } catch (err) {
      setStatus(err.message || "Erro ao enviar arquivo");
    }
  });

  loadConversation();
  loadMessages();
  subscribeCable();
})();
