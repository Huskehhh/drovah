<template>
  <b-modal
      :id=getModalName title="Builds" @on="resetModal" @hidden="resetModal" @Ok="handleOk">
    <b-list-group>
      <b-list-group-item
          v-for="build in getLatestBuilds"
          :key="build.buildNumber">
        Build number: {{ build.buildNumber }} <img :src="getBadgeForBuild(build)" alt="Build status badge"/>
        <b-dropdown id="dropdown-left" text="Downloads" variant="primary" class="m-2" size="sm" v-if="hasFiles(build)">
          <b-dropdown-item
              v-for="file in build.archivedFiles"
              :key="file"
              :href="buildHrefForFile(build, file)">Download {{ file }}</b-dropdown-item>
        </b-dropdown>
      </b-list-group-item>
    </b-list-group>
  </b-modal>
</template>

<script>
export default {
  name: "ProjectModal",
  props: {
    builds: Array,
    projectName: String
  },
  data() {
    return {}
  },
  computed: {
    getModalName() {
      return "builds-modal-" + this.projectName;
    },

    // Provide only the last 10 builds, reversed
    getLatestBuilds() {
      let arr = [];

      for (let i = this.builds.length - 1, index = this.builds.length - 11; i >= 0 && i > index; i--) {
        if (i === index) return arr;
        arr.push(this.builds[i])
      }

      return arr;
    },
  },
  methods: {
    resetModal() {
    },

    getBadgeForBuild: function (build) {
      return "/" + this.projectName + "/" + build.buildNumber + "/badge";
    },

    handleOk: function(bvModalEvt) {
      bvModalEvt.preventDefault();
    },

    buildHrefForFile: function(build, file) {
      return "/" + this.projectName + "/" + build.buildNumber + "/" + file;
    },

    hasFiles: function (build) {
      return build.archivedFiles != null && build.archivedFiles.length > 0;
    },
  },
}
</script>

<style scoped>

</style>