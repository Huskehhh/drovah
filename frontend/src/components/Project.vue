<template>
  <div class="col-lg-3 col-md-6 text-center">
    <div class="mt-5">
      <font-awesome-icon icon="laptop-code" size="3x"/>
      <p class="p mb-2">{{ projectName }} <a v-if="hasLatest()" :href="latestBuild()"><i
          class="fas fa-download"></i></a></p>
      <img :src="badgeUrl()" alt="Build status badge"/>
      <b-button v-b-modal="getModalName" size="sm" id="builds-button">Builds</b-button>
      <project-modal :builds="builds" :project-name="projectName"></project-modal>
    </div>
  </div>
</template>

<script>
import ProjectModal from "ProjectModal";

const API_URL = process.env.VUE_APP_API_URL;

export default {
  name: "Project",
  components: {ProjectModal},
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
      return API_URL + "/api/v1/" + this.projectName + "/badge";
    },
    latestBuild: function () {
      return API_URL + "/api/v1/" + this.projectName + "/latest";
    }
  },

  computed: {
    getModalName() {
      return "builds-modal-" + this.projectName;
    }
  },
}
</script>

<style scoped>
.p {
  font-size: 1rem;
  font-weight: 400;
  line-height: 2.0;
  color: #6c6c6c;
  text-align: center;
}

.text-center {
  text-align: center !important
}

.fa-3x {
  color: #ce2d4b;
}

#builds-button {
  position: relative;
  top: 10px;
}
</style>
