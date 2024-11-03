import Section from "@/components/Section";

export default function TermsAndConditionsPage() {
  return (
    <main className="relative flex h-max w-full items-center justify-center bg-dark px-5 pb-10 pt-32 text-white md:gap-0 md:pt-32">
      <div className="flex w-full max-w-layout flex-col justify-center">
        <Section.ContentContainer className="!flex-auto lg:w-full">
          <div>
            <h1 className="mb-6 text-3xl font-bold">Terms and Conditions</h1>
            <p className="mb-6 text-gray-300">Effective Date: 10/18/2024</p>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">1. Introduction</h2>
              <p className="text-gray-300">
                Welcome to our website. By accessing or using this website, you
                agree to comply with and be bound by the following terms and
                conditions. If you disagree with any part of these terms, please
                do not use our website.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">
                2. Intellectual Property
              </h2>
              <p className="text-gray-300">
                All content, trademarks, and data on this website, including but
                not limited to text, graphics, logos, images, and software, are
                the property of Rich Presence Plus or its content suppliers and
                are protected by international intellectual property laws.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">
                3. Use of the Website
              </h2>
              <p className="text-gray-300">
                You may use this website for personal, non-commercial purposes
                only. You agree not to copy, modify, distribute, or sell any
                content without prior written consent from Rich Presence Plus.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">
                4. Limitation of Liability
              </h2>
              <p className="text-gray-300">
                Rich Presence Plus will not be held liable for any damages
                arising out of the use or inability to use this website or the
                services provided, including but not limited to direct,
                indirect, incidental, punitive, and consequential damages.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">
                5. Changes to the Terms
              </h2>
              <p className="text-gray-300">
                We reserve the right to modify or replace these terms at any
                time. Continued use of the website after any such changes
                constitutes your acceptance of the new terms.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">6. Governing Law</h2>
              <p className="text-gray-300">
                These terms and conditions are governed by and construed in
                accordance with the laws of Argentina, and you irrevocably
                submit to the exclusive jurisdiction of the courts in that
                location.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">7. Contact Us</h2>
              <p className="text-gray-300">
                If you have any questions about these Terms and Conditions,
                please contact us at{" "}
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
