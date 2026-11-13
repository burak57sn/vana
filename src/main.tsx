import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { api } from "./api";
import { I18nProvider, type Language } from "./i18n";
import "./styles.css";

const syncBackend = (language: Language) => {
  api.setLanguage(language).catch(() => {
    // Older backends without the command keep English messages.
  });
};

async function boot() {
  // Outside Tauri (UI work in a browser) the app runs on sample data.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMocks } = await import("./dev/mocks");
    installMocks();
  }
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <I18nProvider onChange={syncBackend}>
        <App />
      </I18nProvider>
    </StrictMode>,
  );
}

void boot();
