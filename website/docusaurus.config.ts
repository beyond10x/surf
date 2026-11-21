import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import {withProductSite} from '@beyond10x/docs-system/product-site';

// The landing page is `product.json`, rendered by the shared docs-system product-site template.
// The pages live in `docs/` and are written by hand from the repository's README.md,
// docs/surf-command.md and AGENTS.md. A page states only what those files state.

const config: Config = {
  title: 'surf',
  tagline: 'A web browser written from scratch, where the specification comes first.',
  favicon: 'img/logo.svg',

  future: {v4: true},
  url: 'https://beyond10x.github.io',
  baseUrl: '/surf/',
  organizationName: 'beyond10x',
  projectName: 'surf',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,
  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',

  markdown: {
    hooks: {onBrokenMarkdownLinks: 'throw'},
  },
  i18n: {defaultLocale: 'en', locales: ['en']},

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: 'docs',
          editUrl: 'https://github.com/beyond10x/surf/tree/main/website/',
        },
        blog: false,
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    navbar: {
      title: 'surf',
      logo: {alt: 'surf', src: 'img/logo.svg'},
      items: [
        {to: '/docs/', label: 'Docs', position: 'left', activeBaseRegex: '^/surf/docs/(?!reference/|roadmap|status)'},
        {to: '/docs/try-it', label: 'Try it', position: 'left'},
        {to: '/docs/reference/surf-command', label: 'Command', position: 'left'},
        {to: '/docs/roadmap', label: 'Roadmap', position: 'left'},
        {to: '/docs/status', label: 'Status', position: 'left'},
        {href: 'https://github.com/beyond10x/surf', label: 'GitHub', position: 'right'},
      ],
    },
    footer: {
      links: [
        {
          title: 'Documentation',
          items: [
            {label: 'What surf is', to: '/docs/'},
            {label: 'Try it', to: '/docs/try-it'},
            {label: 'Principles', to: '/docs/principles'},
            {label: 'How it is built', to: '/docs/how-it-is-built'},
            {label: 'The surf command', to: '/docs/reference/surf-command'},
          ],
        },
        {
          title: 'Project',
          items: [
            {label: 'Roadmap', to: '/docs/roadmap'},
            {label: 'Status', to: '/docs/status'},
            {label: 'Source', href: 'https://github.com/beyond10x/surf'},
          ],
        },
      ],
      copyright: 'A beyond10x project · built with Docusaurus and the docs-system product template.',
    },
  } satisfies Preset.ThemeConfig,
};

export default withProductSite(config, {landing: './product.json', mark: 'S'});
