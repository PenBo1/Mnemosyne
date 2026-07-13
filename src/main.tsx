import React from "react";
import ReactDOM from "react-dom/client";
import { bootstrap } from "./app";
import App from "./app/App";
import { ThemeProvider } from "./lib/theme";
import { I18nProvider } from "./locales/i18n";
import "./styles/index.css";

bootstrap().then(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <ThemeProvider>
        <I18nProvider>
          <App />
        </I18nProvider>
      </ThemeProvider>
    </React.StrictMode>,
  );
});
