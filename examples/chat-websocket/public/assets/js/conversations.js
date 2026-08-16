(() => {
  const listEl = document.getElementById("conversations-list");
  const loadingEl = document.getElementById("conversations-loading");
  const emptyEl = document.getElementById("conversations-empty");
  const dialog = document.getElementById("new-chat-dialog");
  const recipientSelect = document.getElementById("recipient-select");
  const newChatError = document.getElementById("new-chat-error");
  const currentUserId = window.CHAT_USER_ID;

  function peerLabel(conversation) {
    const other = conversation.participants.find((p) => p.id !== currentUserId);
    return other ? other.email : `Conversa #${conversation.id}`;
  }

  function unreadLabel(conversation) {
    if (!conversation.has_unread) return "";
    const count = conversation.unread_count;
    const text =
      count === 1 ? "1 não lida" : `${count} não lidas`;
    return `<span class="unread-badge" aria-label="${text}">${text}</span>`;
  }

  function renderConversations(conversations) {
    loadingEl.hidden = true;
    if (!conversations.length) {
      emptyEl.hidden = false;
      listEl.hidden = true;
      return;
    }
    emptyEl.hidden = true;
    listEl.hidden = false;
    listEl.innerHTML = conversations
      .map(
        (c) => `
      <li class="conversation-item${c.has_unread ? " has-unread" : ""}">
        <a href="/chat/${c.id}">
          <span class="conversation-main">
            <span class="peer-email">${Chat.escapeHtml(peerLabel(c))}</span>
            ${unreadLabel(c)}
          </span>
          <span class="meta">#${c.id}</span>
        </a>
      </li>`
      )
      .join("");
  }

  async function loadConversations() {
    try {
      const conversations = await Chat.request("/conversations");
      renderConversations(conversations);
    } catch (err) {
      loadingEl.textContent = err.message || "Erro ao carregar conversas";
    }
  }

  async function loadUsers() {
    const users = await Chat.request("/users");
    recipientSelect.innerHTML =
      '<option value="">Selecione…</option>' +
      users
        .map(
          (u) =>
            `<option value="${u.id}">${Chat.escapeHtml(u.email)}</option>`
        )
        .join("");
  }

  document.getElementById("sign-out-btn").addEventListener("click", () => {
    Chat.signOut();
  });

  document.getElementById("new-chat-btn").addEventListener("click", async () => {
    newChatError.hidden = true;
    try {
      await loadUsers();
      dialog.showModal();
    } catch (err) {
      alert(err.message || "Erro ao carregar usuários");
    }
  });

  document.getElementById("new-chat-cancel").addEventListener("click", () => {
    dialog.close();
  });

  document.getElementById("new-chat-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    newChatError.hidden = true;
    const recipientId = Number(recipientSelect.value);
    if (!recipientId) return;

    try {
      const conversation = await Chat.request("/conversations", {
        method: "POST",
        body: JSON.stringify({ recipient_id: recipientId }),
      });
      window.location.href = `/chat/${conversation.id}`;
    } catch (err) {
      newChatError.textContent = err.message || "Erro ao criar conversa";
      newChatError.hidden = false;
    }
  });

  loadConversations();
})();
