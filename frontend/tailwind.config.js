/** @type {import('tailwindcss').Config} */
module.exports = {
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
