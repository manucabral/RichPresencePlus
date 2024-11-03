/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/app/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/**/*.{js,ts,jsx,tsx}"
  ],
  theme: {
    extend: {
      colors: {
        "green-1": "#4BF6B6",
        "green-2": "#6ff8c5",
        "green-3": "#81f9cc",
        "green-4": "#93fad3",
        "green-5": "#a5fbdb",
        "green-6": "#b7fbe2",
        "green-7": "#c9fce9",
        "green-8": "#dbfdf0",
        "green-9": "#edfef8",
        "green-10": "#2cf4aa",
        "green-11": "#0ef39d",
        "green-12": "#0bd68a",
        "green-13": "#09b776",
        "green-14": "#089963",
        "green-15": "#067a4f",
        "green-16": "#055c3b",
        "green-17": "#033d27",
        "green-18": "#021f14",
        "pink-1": "#eba4db",
        "pink-2": "#edaedf",
        "pink-3": "#f0b8e3",
        "pink-4": "#f2c2e7",
        "pink-5": "#f4cdeb",
        "pink-6": "#f6d7ef",
        "pink-7": "#f8e1f3",
        "pink-8": "#fbebf7",
        "pink-9": "#fdf5fb",
        "pink-10": "#e27aca",
        "pink-11": "#db5abe",
        "pink-12": "#d43bb1",
        "pink-13": "#bf2a9d",
        "pink-14": "#9f2383",
        "pink-15": "#7f1c68",
        "pink-16": "#5f154e",
        "pink-17": "#400e34",
        "pink-18": "#20071a",
        dark: "#181526",
        "dark-2": "#342f54"
      },
      maxWidth: {
        layout: "1200px"
      }
    }
  },
  plugins: [require("@tailwindcss/forms")]
}
