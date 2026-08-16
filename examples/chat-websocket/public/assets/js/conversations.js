(() => {
  const listEl = document.getElementById("conversations-list");
  const loadingEl = document.getElementById("conversations-loading");
  const emptyEl = document.getElementById("conversations-empty");
  const chatDialog = document.getElementById("new-chat-dialog");
  const groupDialog = document.getElementById("new-group-dialog");
  const recipientSelect = document.getElementById("recipient-select");
  const groupMembersSelect = document.getElementById("group-members-select");
  const groupNameInput = document.getElementById("group-name-input");
  const newChatError = document.getElementById("new-chat-error");
  const newGroupError = document.getElementById("new-group-error");
  const currentUserId = window.CHAT_USER_ID;

  function conversationLabel(conversation) {
    return conversation.display_name || `Conversa #${conversation.id}`;
  }

  function conversationMeta(conversation) {
    if (conversation.kind === "group") {
      const count = conversation.participants.length;
      return `${count} membros`;
    }
    return `#${conversation.id}`;
  }

  function kindBadge(conversation) {
    if (conversation.kind !== "group") return "";
    return '<span class="group-badge">Grupo</span>';
  }

  function unreadLabel(conversation) {
    if (!conversation.has_unread) return "";
    const count = conversation.unread_count;
    const text = count === 1 ? "1 não lida" : `${count} não lidas`;
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
      <li class="conversation-item${c.has_unread ? " has-unread" : ""}${c.kind === "group" ? " is-group" : ""}">
        <a href="/chat/${c.id}">
          <span class="conversation-main">
            <span class="peer-email">${Chat.escapeHtml(conversationLabel(c))}</span>
            ${kindBadge(c)}
            ${unreadLabel(c)}
          </span>
          <span class="meta">${Chat.escapeHtml(conversationMeta(c))}</span>
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

  async function loadUsers(selectEl) {
    const users = await Chat.request("/users");
    selectEl.innerHTML = users
      .map(
        (u) => `<option value="${u.id}">${Chat.escapeHtml(u.email)}</option>`
      )
      .join("");
  }

  function resetDialogErrors() {
    newChatError.hidden = true;
    newGroupError.hidden = true;
  }

  document.getElementById("sign-out-btn").addEventListener("click", () => {
    Chat.signOut();
  });

  document.getElementById("new-chat-btn").addEventListener("click", async () => {
    resetDialogErrors();
    try {
      await loadUsers(recipientSelect);
      recipientSelect.innerHTML =
        '<option value="">Selecione…</option>' + recipientSelect.innerHTML;
      chatDialog.showModal();
    } catch (err) {
      alert(err.message || "Erro ao carregar usuários");
    }
  });

  document.getElementById("new-group-btn").addEventListener("click", async () => {
    resetDialogErrors();
    groupNameInput.value = "";
    try {
      await loadUsers(groupMembersSelect);
      groupDialog.showModal();
    } catch (err) {
      alert(err.message || "Erro ao carregar usuários");
    }
  });

  document.querySelectorAll(".dialog-cancel").forEach((btn) => {
    btn.addEventListener("click", () => btn.closest("dialog").close());
  });

  document.getElementById("new-chat-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    newChatError.hidden = true;
    const recipientId = Number(recipientSelect.value);
    if (!recipientId) return;

    try {
      const conversation = await Chat.request("/conversations", {
        method: "POST",
        body: JSON.stringify({ kind: "direct", recipient_id: recipientId }),
      });
      window.location.href = `/chat/${conversation.id}`;
    } catch (err) {
      newChatError.textContent = err.message || "Erro ao criar conversa";
      newChatError.hidden = false;
    }
  });

  document.getElementById("new-group-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    newGroupError.hidden = true;

    const name = groupNameInput.value.trim();
    const memberIds = Array.from(groupMembersSelect.selectedOptions).map((opt) =>
      Number(opt.value)
    );

    if (!name) {
      newGroupError.textContent = "Informe o nome do grupo";
      newGroupError.hidden = false;
      return;
    }

    if (!memberIds.length) {
      newGroupError.textContent = "Selecione ao menos um membro";
      newGroupError.hidden = false;
      return;
    }

    try {
      const conversation = await Chat.request("/conversations", {
        method: "POST",
        body: JSON.stringify({
          kind: "group",
          name,
          member_ids: memberIds,
        }),
      });
      window.location.href = `/chat/${conversation.id}`;
    } catch (err) {
      newGroupError.textContent = err.message || "Erro ao criar grupo";
      newGroupError.hidden = false;
    }
  });

  loadConversations();
})();
