import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import store from "./store";
import "./tailwind.css"; // Import any global styles

const app = createApp(App);

// Load user state from local storage when the app starts
store.dispatch("loadUserState");

app.use(store);
app.use(router);

app.mount("#app");
