import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'crudo',
  description: 'Configuration-driven JSON APIs backed by SQL.',
  base: '/crudo/',
  cleanUrls: true,
  head: [['link', { rel: 'icon', href: '/crudo/favicon.ico' }]],
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Reference', link: '/reference/configuration' },
      { text: 'Operations', link: '/operations/docker' },
      { text: 'Examples', link: '/examples/curl' },
      { text: 'Limitations', link: '/limitations' },
      { text: 'GitHub', link: 'https://github.com/melonask/crudo' }
    ],
    sidebar: {
      '/guide/': [{ text: 'Guide', items: [{ text: 'Getting started', link: '/guide/getting-started' }, { text: 'Core concepts', link: '/guide/core-concepts' }, { text: 'Configuration', link: '/guide/configuration' }, { text: 'Security', link: '/guide/security' }] }],
      '/reference/': [{ text: 'Reference', items: [{ text: 'Configuration schema', link: '/reference/configuration' }, { text: 'HTTP API', link: '/reference/http-api' }, { text: 'Actions', link: '/reference/actions' }, { text: 'Demo API', link: '/reference/demo-api' }, { text: 'Wallets', link: '/reference/wallets' }, { text: 'Rust API', link: '/reference/rust-api' }] }],
      '/operations/': [{ text: 'Operations', items: [{ text: 'Docker', link: '/operations/docker' }, { text: 'Deployment', link: '/operations/deployment' }] }],
      '/examples/': [{ text: 'Examples', items: [{ text: 'curl lifecycle', link: '/examples/curl' }, { text: 'Custom CRUD API', link: '/examples/custom-crud' }, { text: 'Authentication', link: '/examples/authentication' }, { text: 'Limits and errors', link: '/examples/limits-errors' }, { text: 'SQLite', link: '/examples/sqlite' }, { text: 'PostgreSQL', link: '/examples/postgresql' }] }],
      '/': [{ text: 'More', items: [{ text: 'Limitations & troubleshooting', link: '/limitations' }] }]
    },
    search: { provider: 'local' },
    socialLinks: [{ icon: 'github', link: 'https://github.com/melonask/crudo' }],
    editLink: { pattern: 'https://github.com/melonask/crudo/edit/main/docs/:path', text: 'Edit this page on GitHub' },
    footer: { message: 'Released under the MIT License.', copyright: 'Copyright © 2026 melonask' }
  }
})
