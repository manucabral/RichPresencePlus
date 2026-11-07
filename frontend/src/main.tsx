import { render } from "preact";
import { lazy } from "preact/compat";
import "./index.css";
import Layout from "./Layout";
import DashboardPage from "./features/dashboard/DashboardPage";
import { behaveLikeAnApp } from "./shared/desktop";
import { startLanguage, useT } from "./shared/i18n";
import { Pages, type Route } from "./shared/router";

// The Dashboard opens first; every other page loads when it is visited.
const BrowserPage = lazy(() => import("./features/browser/BrowserPage"));
const CatalogDetailPage = lazy(() => import("./features/catalog/CatalogDetailPage"));
const CatalogPage = lazy(() => import("./features/catalog/CatalogPage"));
const DeveloperPage = lazy(() => import("./features/developer/DeveloperPage"));
const ManualPage = lazy(() => import("./features/manual/ManualPage"));
const PresenceDetailPage = lazy(() => import("./features/presences/PresenceDetailPage"));
const PresencesPage = lazy(() => import("./features/presences/PresencesPage"));
const PrivacyPage = lazy(() => import("./features/privacy/PrivacyPage"));
const SettingsPage = lazy(() => import("./features/settings/SettingsPage"));
const WeekPage = lazy(() => import("./features/week/WeekPage"));

function NotFound() {
  const { t } = useT();
  return <p className="text-sm text-neutral-500">{t("app.notFound")}</p>;
}

const ROUTES: Route[] = [
  ["/", DashboardPage],
  ["/presences", PresencesPage],
  ["/presences/:name", PresenceDetailPage],
  ["/manual", ManualPage],
  ["/week", WeekPage],
  ["/catalog", CatalogPage],
  ["/catalog/:name", CatalogDetailPage],
  ["/browser", BrowserPage],
  ["/privacy", PrivacyPage],
  ["/settings", SettingsPage],
  ["/developer", DeveloperPage],
];

behaveLikeAnApp();
void startLanguage();
render(
  <Layout>
    <Pages routes={ROUTES} fallback={<NotFound />} />
  </Layout>,
  document.getElementById("app")!,
);
