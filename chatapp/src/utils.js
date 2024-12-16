import { invoke } from "@tauri-apps/api/core";

const URL_BASE = "http://localhost:6688/api";
const SSE_URL = "http://localhost:6687/events";

let config = null;
try {
  config = await invoke("read_config");
} catch (error) {
  console.error("Error reading config:", error);
}

const getUrlBase = () => {
  if (config && config.server.chat) {
    return config.server.chat;
  }
  return URL_BASE;
};

const getSseBase = () => {
  if (config && config.server.notification) {
    return config.server.notification;
  }
  return SSE_URL;
};

const initSse = (store) => {
  let sse_base = getSseBase();
  let url = `${sse_base}?token=${store.state.token}`;
  const eventSource = new EventSource(url);

  eventSource.addEventListener("NewMessage", (event) => {
    let data = JSON.parse(event.data);
    console.log("NewMessage: ", event.data);
    delete data.event;
    store.commit("addMessage", { channelId: data.chatId, message: data });
  });

  eventSource.onmessage = (event) => {
    console.log("EventSource message:", event);
    // const data = JSON.parse(event.data);
    // store("addMessage", data);
  };
  eventSource.onerror = (error) => {
    console.error("EventSource failed:", error);
    eventSource.close();
  };

  return eventSource;
};

export { getUrlBase, initSse };
