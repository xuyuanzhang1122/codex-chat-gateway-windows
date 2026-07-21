import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider, ThemeProvider } from "@lobehub/ui";
import { motion } from "motion/react";
import App from "./App";
import { TrayMenu } from "./TrayMenu";
import "./app.css";

const isTrayMenu = new URLSearchParams(window.location.search).has("tray");
document.documentElement.classList.toggle("tray-surface", isTrayMenu);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ConfigProvider motion={motion}>
      <ThemeProvider themeMode="dark" enableGlobalStyle customTheme={{ primaryColor: "purple" }}>
        {isTrayMenu ? <TrayMenu /> : <App />}
      </ThemeProvider>
    </ConfigProvider>
  </React.StrictMode>,
);
