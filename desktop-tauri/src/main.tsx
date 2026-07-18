import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider, ThemeProvider } from "@lobehub/ui";
import { motion } from "motion/react";
import App from "./App";
import "./app.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ConfigProvider motion={motion}>
      <ThemeProvider themeMode="dark" enableGlobalStyle customTheme={{ primaryColor: "purple" }}>
        <App />
      </ThemeProvider>
    </ConfigProvider>
  </React.StrictMode>,
);
