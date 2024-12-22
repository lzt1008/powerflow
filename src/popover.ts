import { MotionPlugin } from '@vueuse/motion'
import { createApp } from 'vue'
import Popover from './Popover.vue'
import './assets/index.css'

createApp(Popover)
  .use(MotionPlugin)
  .mount('#app')
