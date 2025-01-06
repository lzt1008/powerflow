import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import { setup } from './lib/setup'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      component: () => import('./pages/MainContent.vue'),
    },
    {
      path: '/history',
      component: () => import('./pages/History.vue'),
    },
    {
      path: '/history/:id',
      component: () => import('./components/history/HistoryDetail.vue'),
    }
  ],
})

setup(App, (app) => {
  app.use(router)
})
