// SPDX-License-Identifier: GPL-3.0-or-later

export default [
  {
    files: ["adapters/gnome/**/*.js", "adapters/plasma/**/*.js"],
    languageOptions: {
      ecmaVersion: 2024,
      sourceType: "module",
      globals: {
        global: "readonly",
      },
    },
    rules: {
      "eqeqeq": "error",
      "no-undef": "error",
      "no-unused-vars": ["error", {"argsIgnorePattern": "^_"}],
      "prefer-const": "error",
      "semi": ["error", "always"]
    }
  }
];
