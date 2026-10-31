import type { StorybookConfig } from "@storybook/react-vite";

// https://storybook.js.org/docs/get-started/frameworks/react-vite
const config: StorybookConfig = {
  stories: ["../src/**/*.stories.tsx"],
  framework: "@storybook/react-vite",
  core: { disableTelemetry: true },
};

export default config;
