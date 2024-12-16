import { createStore } from "vuex";
import axios from "axios";
import { jwtDecode } from "jwt-decode";
import { getUrlBase, initSse } from "../utils";

export default createStore({
  state: {
    user: null, // User information
    token: null, // Authentication token
    workspace: {}, // Current workspace
    channels: [], // List of channels
    messages: {}, // Messages hashmap, keyed by channel ID
    users: {}, // Users hashmap under the current workspace, keyed by user ID
    activeChannel: null,
    sse: null,
  },
  mutations: {
    setSse(state, sse) {
      state.sse = sse;
    },
    setUser(state, user) {
      state.user = user;
    },
    setToken(state, token) {
      state.token = token;
    },
    setWorkspace(state, workspace) {
      state.workspace = workspace;
    },
    setChannels(state, channels) {
      state.channels = channels;
    },
    setUsers(state, users) {
      state.users = users;
    },
    setMessages(state, { channelId, messages }) {
      state.messages[channelId] = messages;
    },
    setMessagesForChannel(state, { channelId, messages }) {
      state.messages[channelId] = messages;
    },
    setActiveChannel(state, channelId) {
      const channel = state.channels.find(
        (channel) => channel.id === channelId
      );
      state.activeChannel = channel;
    },
    addChannel(state, channel) {
      state.channels.push(channel);
      state.messages[channel.id] = [];
    },
    addMessage(state, { channelId, message }) {
      if (state.messages[channelId]) {
        // we should prepend the message to the list
        state.messages[channelId].unshift(message);
      } else {
        state.messages[channelId] = [message];
      }
    },
    loadUserState(state) {
      const user = localStorage.getItem("user");
      const token = localStorage.getItem("token");
      const workspace = localStorage.getItem("workspace");
      const channels = localStorage.getItem("channels");
      // we do not store messages in local storage, so this is always empty
      const messages = localStorage.getItem("messages");
      const users = localStorage.getItem("users");

      if (user) {
        state.user = JSON.parse(user);
      }
      if (token) {
        state.token = token;
      }
      if (workspace) {
        state.workspace = JSON.parse(workspace);
      }
      if (channels) {
        state.channels = JSON.parse(channels);
      }
      if (messages) {
        state.messages = JSON.parse(messages);
      }
      if (users) {
        state.users = JSON.parse(users);
      }
    },
  },
  actions: {
    initSse({ state, commit }) {
      if (state.sse) {
        state.sse.close();
      }
      const sse = initSse(this);
      commit("setSse", sse);
    },
    closeSse({ state, commit }) {
      if (state.sse) {
        state.sse.close();
        commit("setSse", null);
      }
    },
    async signup({ state, commit }, { workspace, full_name, email, password }) {
      try {
        const response = await axios.post(`${getUrlBase()}/signup`, {
          workspace,
          full_name,
          email,
          password,
        });

        const user = await loadState(response, this, commit);
        return user;
      } catch (error) {
        console.error("Signup failed", error);
        throw error;
      }
    },
    async signin({ commit }, { email, password }) {
      try {
        const response = await axios.post(`${getUrlBase()}/signin`, {
          email,
          password,
        });

        const user = await loadState(response, this, commit);
        return user;
      } catch (error) {
        console.error("Signin failed", error);
        throw error;
      }
    },
    logout({ commit }) {
      // Clear local storage and state
      localStorage.removeItem("user");
      localStorage.removeItem("token");
      localStorage.removeItem("workspace");
      localStorage.removeItem("channels");
      localStorage.removeItem("messages");

      commit("setUser", null);
      commit("setToken", null);
      commit("setWorkspace", "");
      commit("setChannels", []);
      commit("setMessages", {});

      // close sse
      this.dispatch("closeSse");
    },
    setActiveChannel({ commit }, channel) {
      commit("setActiveChannel", channel);
    },
    addChannel({ commit }, channel) {
      commit("addChannel", channel);

      // Update the channels and messages in local storage
      localStorage.setItem("channels", JSON.stringify(this.state.channels));
      localStorage.setItem("messages", JSON.stringify(this.state.messages));
    },
    async fetchMessagesForChannel({ state, commit }, channelId) {
      if (
        !state.messages[channelId] ||
        state.messages[channelId].length === 0
      ) {
        try {
          const response = await axios.get(
            `${getUrlBase()}/chats/${channelId}/messages`,
            {
              headers: {
                Authorization: `Bearer ${state.token}`,
              },
            }
          );
          let messages = response.data;
          // messages = messages.map((message) => {
          //   const user = state.users[message.senderId];
          //   return { ...message, sender: user };
          // });
          commit("setMessages", { channelId, messages });
        } catch (error) {
          console.error(
            `Failed to fetch messages for channel ${channelId}`,
            error
          );
        }
      }
    },
    async sendMessage({ state, commit }, payload) {
      try {
        const response = await axios.post(
          `${getUrlBase()}/chats/${payload.chatId}`,
          payload,
          {
            headers: {
              Authorization: `Bearer ${this.state.token}`,
            },
          }
        );

        console.log("Message sent", response.data);
        commit("addMessage", {
          channelId: payload.chatId,
          message: response.data,
        });
      } catch (error) {
        console.error("Failed to send message", error);
        throw error;
      }
    },
    addMessage({ commit }, { channelId, message }) {
      commit("addMessage", { channelId, message });
    },
    loadUserState({ commit }) {
      commit("loadUserState");

      // if user is already logged in, then init sse
      if (this.state.token) {
        this.dispatch("initSse");
      }
    },
  },
  getters: {
    isAuthenticated(state) {
      return !!state.token;
    },
    getUser(state) {
      return state.user;
    },
    getUserById: (state) => (id) => {
      return state.users[id];
    },
    getWorkspace(state) {
      return state.workspace;
    },
    getChannels(state) {
      // filter out channels that type == 'single'
      return state.channels.filter((channel) => channel.type !== "single");
    },
    getSingleChannels(state) {
      const channels = state.channels.filter(
        (channel) => channel.type === "single"
      );
      // return channel member that is not the current user
      return channels.map((channel) => {
        const id = channel.members.find((id) => id !== state.user.id);
        channel.recipient = state.users[id];
        return channel;
      });
    },
    getChannelMessages: (state) => (channelId) => {
      return state.messages[channelId] || [];
    },
    getMessagesForActiveChannel(state) {
      if (!state.activeChannel) {
        return [];
      }
      return state.messages[state.activeChannel.id] || [];
    },
  },
});

async function loadState(response, self, commit) {
  const token = response.data.token;
  const user = jwtDecode(token);
  const workspace = { id: user.wsId, name: user.wsName };

  try {
    // fetch all workspace users
    const usersResp = await axios.get(`${getUrlBase()}/users`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    const users = usersResp.data;
    const usersMap = {};
    users.forEach((user) => {
      usersMap[user.id] = user;
    });

    // fetch all my channels
    const chatsResp = await axios.get(`${getUrlBase()}/chats`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    const channels = chatsResp.data;

    localStorage.setItem("user", JSON.stringify(user));
    localStorage.setItem("token", token);
    localStorage.setItem("workspace", JSON.stringify(workspace));
    localStorage.setItem("users", JSON.stringify(usersMap));
    localStorage.setItem("channels", JSON.stringify(channels));

    commit("setUser", user);
    commit("setToken", token);
    commit("setWorkspace", workspace);
    commit("setChannels", channels);
    commit("setUsers", usersMap);

    // call initSse action
    await self.dispatch("initSse");

    return user;
  } catch (error) {
    console.error("Failed to load user state", error);
    throw error;
  }
}
