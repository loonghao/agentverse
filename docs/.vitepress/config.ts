import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'AgentVerse',
  description: 'The Universal Marketplace for AI Agent Ecosystems — publish, discover and compose AI skills, agents, workflows, souls and more.',
  base: '/agentverse/',

  // Ignore dead links to localhost (common in code examples)
  ignoreDeadLinks: [
    /^http:\/\/localhost/,
    /^https?:\/\/127\.0\.0\.1/,
  ],

  head: [
    ['link', { rel: 'icon', href: '/agentverse/favicon.ico' }],
    ['meta', { name: 'theme-color', content: '#6366f1' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:site_name', content: 'AgentVerse' }],
  ],

  themeConfig: {
    logo: '🌌',
    siteTitle: 'AgentVerse',

    nav: [
      { text: 'Guide', link: '/guide/introduction' },
      { text: 'CLI', link: '/cli/' },
      { text: 'Server', link: '/server/' },
      { text: 'Storage', link: '/storage/' },
      { text: 'Manifest', link: '/manifest/format' },
      {
        text: 'GitHub',
        link: 'https://github.com/loonghao/agentverse',
      },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Introduction', link: '/guide/introduction' },
            { text: 'Quick Start', link: '/guide/quick-start' },
          ],
        },
      ],
      '/cli/': [
        {
          text: 'CLI Reference',
          items: [
            { text: 'Overview', link: '/cli/' },
            { text: 'Installation', link: '/cli/installation' },
            { text: 'Configuration', link: '/cli/configuration' },
            { text: 'Authentication', link: '/cli/auth' },
          ],
        },
        {
          text: 'Commands',
          items: [
            { text: 'Discovery', link: '/cli/discovery' },
            { text: 'Publishing', link: '/cli/publishing' },
            { text: 'Social', link: '/cli/social' },
            { text: 'Agent (M2M)', link: '/cli/agent' },
          ],
        },
      ],
      '/server/': [
        {
          text: 'Server',
          items: [
            { text: 'Overview', link: '/server/' },
            { text: 'Configuration', link: '/server/configuration' },
            { text: 'Deployment', link: '/server/deployment' },
            { text: 'API Reference', link: '/server/api' },
          ],
        },
      ],
      '/storage/': [
        {
          text: 'Storage Backends',
          items: [
            { text: 'Overview', link: '/storage/' },
            { text: 'Local Filesystem', link: '/storage/local' },
            { text: 'S3 / COS / MinIO / R2', link: '/storage/s3-compatible' },
            { text: 'GitHub Releases', link: '/storage/github-releases' },
            { text: 'Custom HTTP', link: '/storage/custom' },
            { text: 'BK-Repo (蓝鲸)', link: '/storage/bk-repo' },
          ],
        },
      ],
      '/manifest/': [
        {
          text: 'Manifest',
          items: [
            { text: 'Format Reference', link: '/manifest/format' },
          ],
        },
      ],
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/loonghao/agentverse' },
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2025 AgentVerse Contributors',
    },

    search: {
      provider: 'local',
    },

    editLink: {
      pattern: 'https://github.com/loonghao/agentverse/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },
  },
})
