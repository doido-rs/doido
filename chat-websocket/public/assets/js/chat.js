const Chat = (() => {
  async function request(path, options = {}) {
    const res = await fetch(path, {
      credentials: "same-origin",
      headers: {
        Accept: "application/json",
        ...(options.body && !(options.body instanceof FormData)
          ? { "Content-Type": "application/json" }
          : {}),
        ...options.headers,
      },
      ...options,
    });

    if (res.status === 401 || res.status === 403) {
      window.location.href = "/login";
      throw new Error("Não autorizado");
    }

    const contentType = res.headers.get("content-type") || "";
    const data = contentType.includes("application/json")
      ? await res.json().catch(() => null)
      : null;

    if (!res.ok) {
      const message =
        (data && (data.error || data.message)) ||
        `Erro ${res.status}`;
      throw new Error(message);
    }

    return data;
  }

  async function signIn(email, password) {
    return request("/users/sign_in", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    });
  }

  async function signOut() {
    await request("/users/sign_out", { method: "DELETE" });
    window.location.href = "/login";
  }

  function cableUrl() {
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${proto}//${window.location.host}/cable`;
  }

  function formatTime(iso) {
    if (!iso) return "";
    const d = new Date(iso);
    return d.toLocaleString("pt-BR", {
      day: "2-digit",
      month: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  return { request, signIn, signOut, cableUrl, formatTime, escapeHtml };
})();

window.Chat = Chat;
