import { MotionPlugin } from '@vueuse/motion'
import { createApp } from 'vue'
import App from './App.vue'
import './assets/index.css'

createApp(App)
  .use(MotionPlugin)
  .mount('#app')
