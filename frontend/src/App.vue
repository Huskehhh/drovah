<template>
  <div class="masthead" id="app">
    <div class="container h-100">
      <div class="row h-100 align-items-center justify-content-center text-center">
        <div class="col-lg-10 align-self-end">
          <h1 class="text-uppercase text-white font-weight-bold">Projects</h1>
          <hr class="divider my-4"/>
        </div>
        <div class="col-lg-8 align-self-baseline">
          <div class="row" v-if="!loading">
            <project
                id="project"
                :key="project.project"
                v-bind:builds="project.builds"
                v-bind:project-name="project.project"
                v-for="project in projects"></project>
          </div>
          <div class="fixed-bottom">
            <p><a href="https://github.com/Huskehhh/drovah/">drovah</a> - A
              continuous integration service written in Rust!</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import Project from './components/Project.vue'

const API_URL = process.env.VUE_APP_API_URL;

export default {
  name: 'App',
  components: {
    Project
  },
  data: function () {
    return {
      projects: [],
      loading: false,
    };
  },

  mounted() {
    this.$http
        .get(API_URL + '/api/v1/projects')
        .then(response => (this.projects = response.data.projects))
        .finally(() => {
          this.loading = false;
        })
  },
}
</script>

<style>
@import url('https://fonts.googleapis.com/css2?family=Merriweather+Sans&display=swap');

body, html {
  height: 100%;
  background: url(assets/bg-masthead.jpg) no-repeat center;
  background-size: cover;
}

#app {
  position: relative;
  top: 10%;
  font-family: 'Merriweather Sans', sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #1b252f;
}

a {
  color: #f4623a;
  text-decoration: none;
  background-color: transparent
}

a:hover {
  color: #d6370c;
  text-decoration: underline
}

hr.divider {
  max-width: 3.25rem;
  border-width: .2rem;
  border-color: #f4623a
}

.my-4 {
  margin-top: 1.5rem !important
}

.my-4 {
  margin-bottom: 1.5rem !important
}

hr {
  box-sizing: content-box;
  height: 0;
  overflow: visible
}

h1, h2, h3, h4, h5, h6 {
  margin-top: 0;
  margin-bottom: .5rem
}

small {
  font-size: 80%
}

a {
  color: #f4623a;
  text-decoration: none;
  background-color: transparent
}

a:hover {
  color: #d6370c;
  text-decoration: underline
}

.text-white {
  color: #fff !important
}

.font-weight-bold {
  font-weight: 700 !important
}

.align-items-center {
  align-items: center !important
}

.justify-content-center {
  justify-content: center !important
}

.justify-content-center {
  justify-content: center !important
}

.align-items-center {
  align-items: center !important
}

.align-self-end {
  align-self: flex-end !important
}

.align-self-baseline {
  align-self: baseline !important
}

.row {
  display: flex;
  flex-wrap: wrap;
  margin-right: -15px;
  margin-left: -15px
}

.col-lg-10, .col-lg-8 {
  position: relative;
  width: 100%;
  padding-right: 15px;
  padding-left: 15px
}

.col-lg-8 {
  flex: 0 0 66.6666666667%;
  max-width: 66.6666666667%
}

.col-lg-10 {
  flex: 0 0 83.3333333333%;
  max-width: 83.3333333333%
}

.fixed-bottom {
  position: fixed;
  bottom: 0;
  align-self: center;
}

#project {
  padding: 2rem;
}
</style>
