/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./frontend/index.html",
    "./frontend/src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: ['class', '[data-theme="dark"]'],
  theme: {
    extend: {
      colors: {
        accent: 'var(--accent)',
        'accent-hover': 'var(--accent-hover)',
        'accent-subtle': 'var(--accent-subtle)',
      },
      backgroundColor: {
        base: 'var(--bg-base)',
        surface: 'var(--bg-surface)',
        elevated: 'var(--bg-elevated)',
        sidebar: 'var(--bg-sidebar)',
        hover: 'var(--bg-hover)',
        active: 'var(--bg-active)',
      },
      textColor: {
        primary: 'var(--text-primary)',
        secondary: 'var(--text-secondary)',
        disabled: 'var(--text-disabled)',
        inverse: 'var(--text-inverse)',
      },
      borderColor: {
        DEFAULT: 'var(--border)',
        focus: 'var(--border-focus)',
      },
      borderRadius: {
        sm: 'var(--radius-sm)',
        md: 'var(--radius-md)',
        lg: 'var(--radius-lg)',
      },
      boxShadow: {
        sm: 'var(--shadow-sm)',
        md: 'var(--shadow-md)',
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'sans-serif'],
        mono: ['JetBrains Mono', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
};
