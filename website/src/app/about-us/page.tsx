import Section from "@/components/Section";

export default function AboutUsPage() {
  return (
    <main className="relative flex h-max w-full items-center justify-center bg-dark px-5 pb-10 pt-32 text-white md:gap-0 md:pt-32">
      <div className="flex w-full max-w-layout flex-col justify-center">
        <Section.ContentContainer className="!flex-auto lg:w-full">
          <div>
            <h1 className="mb-6 text-3xl font-bold">About Us</h1>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">Who We Are</h2>
              <p className="text-gray-300">
                We are a dedicated team committed to providing top-quality
                Discord presence programs. Our goal is to offer tools that
                enhance your Discord experience, allowing you to showcase your
                activities in unique and personalized ways.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">Our Mission</h2>
              <p className="text-gray-300">
                Our mission is to help users express themselves and their
                interests through their Discord presence. We believe in creating
                tools that are easy to use, customizable, and accessible to
                everyone. We are passionate about continuously improving our
                services and providing users with innovative features.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">Our Values</h2>
              <ul className="list-inside list-disc text-gray-300">
                <li>
                  <b>Innovation:</b> Continuously improving our products to meet
                  the needs of our users.
                </li>
                <li>
                  <b>Accessibility:</b> Ensuring our tools are available for all
                  types of users.
                </li>
                <li>
                  <b>Customer Satisfaction:</b> Listening to feedback and making
                  enhancements that matter.
                </li>
                <li>
                  <b>Transparency:</b> Being clear about our practices and
                  products.
                </li>
              </ul>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">Why Choose Us?</h2>
              <p className="text-gray-300">
                We provide reliable and user-friendly Discord presence programs.
                With a focus on constant innovation, we ensure that our users
                always have access to the best tools and features. Whether
                you&apos;re a casual Discord user or a community leader, our
                services are designed to fit your needs.
              </p>
            </section>
            <section className="mb-8">
              <h2 className="mb-4 text-2xl font-semibold">Contact Us</h2>
              <p className="text-gray-300">
                Have questions or want to learn more about us? Feel free to
                reach out! You can contact us at{" "}
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
