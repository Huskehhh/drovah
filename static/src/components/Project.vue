<template>
  <div class="col-lg-3 col-md-6 text-center">
    <div class="mt-5">
      <font-awesome-icon icon="laptop-code" size="4x"/>
      <p class="p mb-2">{{ projectName }} <a v-if="hasLatest()" :href="latestBuild()"><i
          class="fas fa-download"></i></a></p>
      <hr class="divider my-4"/>
      <img :src="badgeUrl()" alt="Build status badge"/>
    </div>
  </div>
</template>

<script>
export default {
  name: "Project",
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
}
</script>

<style scoped>
.p {
  margin: 0;
  font-size: 2rem;
  font-weight: 400;
  line-height: 2.0;
  color: #6c6c6c;
  text-align: center;
}

.text-center {
  text-align: center !important
}

.fa-4x {
  color: #ce2d4b;
}
</style>