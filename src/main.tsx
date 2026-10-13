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
