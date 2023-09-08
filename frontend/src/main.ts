import './assets/scss/app.scss'

import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import Oruga from "@oruga-ui/oruga-next"
import { bulmaConfig } from "@oruga-ui/theme-bulma"

const app = createApp(App)

app.use(createPinia())
app.use(router)

app.use(Oruga, bulmaConfig);

app.mount('#app')
