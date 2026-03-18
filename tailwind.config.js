/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./crates/prometheus-ui/src/**/*.rs",
    "./crates/prometheus-server/src/**/*.rs",
    "./index.html",
    "./templates/**/*.html",
  ],
  theme: {
    extend: {
      /* ── NexusEdge Color Palette ────────────────────── */
      colors: {
        primary: {
          DEFAULT: "#14b8a6",
          dark: "#0d9488",
          light: "rgba(20, 184, 166, 0.20)",
          hover: "rgba(20, 184, 166, 0.90)",
          50: "#f0fdfa",
          100: "#ccfbf1",
          200: "#99f6e4",
          300: "#5eead4",
          400: "#2dd4bf",
          500: "#14b8a6",
          600: "#0d9488",
          700: "#0f766e",
          800: "#115e59",
          900: "#134e4a",
        },
        cream: {
          DEFAULT: "#FFFDF7",
          50: "#FFFDF7",
        },
        "bg-cream": "#FFFDF7",
        "bg-off-white": "#FAF8F5",
        "bg-warm-beige": "#F5EDE8",
        "border-tan": "#E8D4C4",
        "border-tan-hover": "#D4B8A8",
        terracotta: "#C4A484",
        russet: "#C2714F",
        /* Equipment type colors */
        "equip-air-handler": "#14b8a6",
        "equip-boiler": "#b91c1c",
        "equip-pump": "#06b6d4",
        "equip-chiller": "#0ea5e9",
        "equip-fan-coil": "#3b82f6",
        "equip-steam": "#8b5cf6",
        /* Status colors */
        success: "#22c55e",
        warning: "#f97316",
        error: "#dc2626",
        info: "#3b82f6",
      },

      /* ── Typography ─────────────────────────────────── */
      fontFamily: {
        sans: ["Inter", "system-ui", "-apple-system", "sans-serif"],
        mono: ["JetBrains Mono", "Fira Code", "Consolas", "monospace"],
      },

      /* ── Spacing & Sizing ───────────────────────────── */
      borderRadius: {
        card: "12px",
        btn: "8px",
        badge: "6px",
      },

      /* ── Shadows ────────────────────────────────────── */
      boxShadow: {
        card: "0 1px 3px 0 rgba(0, 0, 0, 0.04), 0 1px 2px -1px rgba(0, 0, 0, 0.03)",
        "card-hover": "0 4px 6px -1px rgba(0, 0, 0, 0.06), 0 2px 4px -2px rgba(0, 0, 0, 0.04)",
        modal: "0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.1)",
      },

      /* ── Animations ─────────────────────────────────── */
      keyframes: {
        "fade-in": {
          "0%": { opacity: "0", transform: "translateY(4px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        "slide-in-right": {
          "0%": { opacity: "0", transform: "translateX(16px)" },
          "100%": { opacity: "1", transform: "translateX(0)" },
        },
        "pulse-dot": {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.4" },
        },
        "progress-bar": {
          "0%": { width: "0%" },
          "100%": { width: "100%" },
        },
      },
      animation: {
        "fade-in": "fade-in 0.2s ease-out",
        "slide-in-right": "slide-in-right 0.25s ease-out",
        "pulse-dot": "pulse-dot 1.5s ease-in-out infinite",
        "progress-bar": "progress-bar 2s ease-in-out",
      },

      /* ── Custom widths ──────────────────────────────── */
      width: {
        sidebar: "260px",
        "sidebar-collapsed": "64px",
      },
      minHeight: {
        card: "120px",
      },
    },
  },
  plugins: [],
};
