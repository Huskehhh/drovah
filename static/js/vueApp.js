Vue.component('project', {
    props: {
        projectName: String,
        builds: Array,
    },
    methods: {
        hasLatest: function () {
            let result = false;
            for (let i = 0; i < this.builds.length; i++) {
                let obj = this.builds[i];
                if (obj.archivedFiles != null) {
                    result = true;
                }
            }
            return result;
        },
        badgeUrl: function () {
            return "/" + this.projectName + "/badge";
        },
        latestBuild: function () {
            return "/" + this.projectName + "/latest";
        }
    },
    template: '<div class="col-lg-3 col-md-6 text-center">\n' +
        '                        <div class="mt-5" style="padding-bottom: 30%;">\n' +
        '                            <i class="fas fa-4x fa-laptop-code text-primary mb-4"></i>\n' +
        '                            <p class="p mb-2">{{ projectName }} <a v-if="hasLatest()" :href="latestBuild()"><i class="fas fa-download"></i></a></p>\n' +
        '                            <img :src="badgeUrl()" alt="Build status badge"/>\n' +
        '                        </div>\n' +
        '                    </div>'
})

const Project = {
    template: '<div>Project tapped {{ $route.params.project }}</div>'
}

const routes = [
    { path: '/:project', component: Project },
]

const router = new VueRouter({
    routes
})

const app = new Vue({
    router,
    el: '#app',
    data: {
        loading: false,
        projects: []
    },

    mounted() {
        axios
            .get('/api/projects')
            .then(response => (this.projects = response.data.projects))
            .finally(() => {
                this.loading = false;
            })
    },

    methods: {},
})