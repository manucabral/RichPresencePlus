import Header from "@/components/Header";
import "./globals.css";
import { Commissioner } from "next/font/google";
import Footer from "@/sections/Footer";
import { Analytics } from "@vercel/analytics/react";
const inter = Commissioner({ subsets: ["latin"] });

export const metadata = {
  title: "Rich Presence Plus",
  description:
    "Show your browser activity on your discord presence with Rich Presence Plus",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <link rel="icon" href="/images/rich-presence-plus-logo.svg" />
        <meta name="google-adsense-account" content="ca-pub-1702796910862555" />
        <script
          async
          src="https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js?client=ca-pub-1702796910862555"
          crossOrigin="anonymous"
        ></script>
      </head>
      <body className={inter.className} suppressHydrationWarning={true}>
        <Header />
        {children}
        <Analytics />
        <Footer />
      </body>
    </html>
  );
}
