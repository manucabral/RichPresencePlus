import Section from "@/components/Section";

export default function PrivacyPolicyPage() {
  return (
    <main className="relative flex h-max w-full items-center justify-center bg-dark px-5 pb-10 pt-32 text-white md:gap-0 md:pt-32">
      <div className="flex w-full max-w-layout flex-col justify-center">
        <Section.ContentContainer className="!flex-auto lg:w-full">
          <div>
            <h1 className="mb-6 text-3xl font-bold">Privacy Policy</h1>
            <p className="mb-6 text-gray-300">Effective Date: 10/18/2024</p>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">
                1. Information We Collect
              </h2>
              <p className="mb-4 text-gray-300">
                We do not directly collect any personal information from users
                on this website. However, we may use third-party services, such
                as Google AdSense, which may collect data to provide relevant
                advertisements.
              </p>
              <h3 className="mb-2 text-xl font-semibold">
                a. Automatically Collected Data:
              </h3>
              <ul className="list-inside list-disc text-gray-300">
                <li>IP address.</li>
                <li>Device information.</li>
                <li>
                  Information about your interaction with our website (pages
                  visited, time spent, etc.).
                </li>
              </ul>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">2. Google AdSense</h2>
              <p className="text-gray-300">
                We use Google AdSense to display ads on our site. Google may use
                technologies like tracking mechanisms to show ads based on your
                interests and browsing habits.
              </p>
              <p className="mt-4 text-gray-300">
                For more information on how Google collects and uses your data,
                please visit the{" "}
                <a
                  href="https://policies.google.com/privacy"
                  className="text-blue-600 underline"
                >
                  Google Privacy Policy
                </a>
                .
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">
                3. Links to Third-Party Websites
              </h2>
              <p className="text-gray-300">
                Our site may contain links to external websites. We are not
                responsible for the privacy practices or the content of these
                websites. We recommend reviewing the privacy policies of each
                third-party site you visit.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">4. Security</h2>
              <p className="text-gray-300">
                We are committed to maintaining the security of any information
                collected through our site. However, please note that no method
                of transmission over the Internet or electronic storage is
                completely secure, and we cannot guarantee absolute security.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">
                5. Changes to This Privacy Policy
              </h2>
              <p className="text-gray-300">
                We reserve the right to update this Privacy Policy at any time.
                Any significant changes will be posted on this page, and the
                date of the latest revision will be indicated.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">6. Contact Us</h2>
              <p className="text-gray-300">
                If you have any questions or concerns regarding this Privacy
                Policy, please contact us at{" "}
                <a
                  href="mailto:support@richpresenceplus.com"
                  className="text-blue-600 underline"
                >
                  support@richpresenceplus.com
                </a>
              </p>
            </section>
          </div>
        </Section.ContentContainer>
      </div>
    </main>
  );
}
