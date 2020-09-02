import Vue from 'vue'
import App from './App.vue'

// Font awesome
import {library} from '@fortawesome/fontawesome-svg-core'
import {faLaptopCode} from '@fortawesome/free-solid-svg-icons'
import {FontAwesomeIcon} from '@fortawesome/vue-fontawesome'
import VueAxios from 'vue-axios'
import axios from 'axios'

library.add(faLaptopCode)
Vue.component('font-awesome-icon', FontAwesomeIcon)
Vue.use(VueAxios, axios)

Vue.config.productionTip = false

new Vue({
    render: h => h(App),
}).$mount('#app')
