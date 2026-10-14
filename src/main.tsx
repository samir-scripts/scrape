import "@fontsource-variable/inter/wght.css";
import "@fontsource-variable/jetbrains-mono/wght.css";
import "@fontsource-variable/geist-mono/wght.css";
import "@fontsource-variable/fira-code/wght.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "@fontsource/ibm-plex-mono/latin-700.css";
import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { needsDesktop } from "./lib/runtime";
import "./styles/index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5,
      refetchOnWindowFocus: false,
    },
  },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      {needsDesktop() ? (
        <main className="p-8 text-[var(--color-ink)] bg-[var(--color-paper)]">
          <h1 className="text-xl font-bold">Scrape</h1>
          <p className="my-4">Live scraping and saved jobs require the desktop application.</p>
          <a className="underline" href="?demo=1">Explore a demonstration with fictional postings</a>
        </main>
      ) : <App />}
    </QueryClientProvider>
  </StrictMode>
);
