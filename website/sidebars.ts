import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

/**
 * One hand-written sidebar: what surf is and a first run, how it is meant to work, the command
 * reference, and where the project stands.
 */
const sidebars: SidebarsConfig = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Start here',
      collapsed: false,
      items: ['index', 'try-it'],
    },
    {
      type: 'category',
      label: 'Concepts',
      collapsed: false,
      items: ['principles', 'what-works', 'how-it-is-built'],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: false,
      items: ['reference/surf-command'],
    },
    {
      type: 'category',
      label: 'Project',
      collapsed: false,
      items: ['roadmap', 'status'],
    },
  ],
};

export default sidebars;
