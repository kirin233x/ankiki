import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import "./mobile.css";
import { isMobile } from "./platform";

if (isMobile) document.documentElement.classList.add("mobile");

// 防止误触浏览器默认行为（右键菜单、拖拽打开文件）
window.addEventListener("dragover", (e) => e.preventDefault());
window.addEventListener("drop", (e) => e.preventDefault());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
