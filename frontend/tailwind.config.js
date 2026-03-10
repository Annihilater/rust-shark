/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: [
    "./index.html",
    "./src/**/*.rs",
  ],
  theme: {
    extend: {
      colors: {
        gray: {
          750: '#2d3748',
        }
      }
    },
  },
  plugins: [],
}
