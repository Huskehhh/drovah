Vue.component('project', {
    props: {
        projectName: String,
        buildStatus: String,
        latestBuild: String,
        hasFiles: Boolean
    },
    template: '<div class="col-lg-3 col-md-6 text-center">\n' +
        '                        <div class="mt-5" style="padding-bottom: 30%;">\n' +
        '                            <i class="fas fa-4x fa-laptop-code text-primary mb-4"></i>\n' +
        '                            <p class="p mb-2">{{ projectName }} <a v-if="hasFiles" v-bind:href="latestBuild"><i class="fas fa-download"></i></a></p>\n' +
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
            .finally(() => {
                this.loading = false;
            })
    },

    methods: {
        badgeUrlForProject: function (projectName) {
            return "/" + projectName + "/badge";
        },
        latestBuildForProject: function (projectName) {
            return "/" + projectName + "/latest";
        }
    },
})