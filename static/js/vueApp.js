Vue.component('project', {
    props: {
        projectName: String,
        buildStatus: String
    },
    template: '<div class="col-lg-3 col-md-6 text-center">\n' +
        '                        <div class="mt-5">\n' +
        '                            <i class="fas fa-4x fa-laptop-code text-primary mb-4"></i>\n' +
        '                            <h3 class="h4 mb-2">{{ projectName }}</h3>\n' +
        '                            <img v-bind:src="buildStatus" alt="Build status badge"/>\n' +
        '                        </div>\n' +
        '                    </div>'
})

new Vue({
    el: '#app',
    data: {
        loading: false,
        projects: []
    },

    mounted() {
        axios
            .get('/api/projects')
            .then(response => (this.projects = response.data.projects))
            .finally(() => this.loading = false)
    },

    methods: {
        badgeUrlForProject: function (projectName) {
            return "/" + projectName + "/badge";
        }
    },
})