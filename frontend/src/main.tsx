import { render } from "preact";
import { HashRouter, Routes, Route } from "react-router";
import "./index.css";
import App from "./app";
import Layout from "./Layout";
import BrowserPage from "./features/browser/pages/BrowserPage";
import InstalledPresencesPage from "./features/presences/pages/InstalledPresencesPage";
import RemotePresencesPage from "./features/presences/pages/RemotePresencesPage";
import NetworkPage from "./features/network/pages/NetworkPage";
import DiscordPage from "./features/discord/pages/DiscordPage";
import SettingsPage from "./features/settings/pages/SettingsPage";

render(
  <HashRouter>
    <Routes>
      <Route element={<Layout />}>
        <Route index path="/" element={<App />} />
        <Route path="browser" element={<BrowserPage />} />
        <Route
          path="installed-presences"
          element={<InstalledPresencesPage />}
        />
        <Route path="remote-presences" element={<RemotePresencesPage />} />
        <Route path="network" element={<NetworkPage />} />
        <Route path="discord" element={<DiscordPage />} />
        <Route path="settings" element={<SettingsPage />} />
        {/* <Route path="custom-presence" element={<CustomPresencePage />} /> */}
        <Route path="*" element={<div>404 Not Found</div>} />
      </Route>
    </Routes>
  </HashRouter>,
  document.getElementById("app")!,
);
