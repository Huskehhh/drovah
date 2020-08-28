Vue.component('project', {
    props: {
        projectName: String,
        builds: Array,
    },
    methods: {
        hasLatest: function () {
            console.log("Called latest for " + this.projectName);
            return this.builds.length === 1;
        },
        badgeUrl: function () {
            console.log("Called badge");
            return "/" + this.projectName + "/badge";
        },
        latestBuild: function () {
            console.log("Called latest");
            return "/" + this.projectName + "/latest";
        }
    },
    template: '<div class="col-lg-3 col-md-6 text-center">\n' +
        '                        <div class="mt-5" style="padding-bottom: 30%;">\n' +
        '                            <i class="fas fa-4x fa-laptop-code text-primary mb-4"></i>\n' +
        '                            <p class="p mb-2">{{ projectName }} <a v-if="hasLatest()" v-bind:href="latestBuild()"><i class="fas fa-download"></i></a></p>\n' +
        '                            <img v-bind:src="badgeUrl()" alt="Build status badge"/>\n' +
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

    methods: {},
})