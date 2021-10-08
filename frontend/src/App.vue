<template>
  <div id="app">
    <sidebar />
    <router-view class="content" />
  </div>
</template>
<script>
import { Component, Vue } from "vue-property-decorator";
import Sidebar from "@/components/Sidebar.vue";

@Component({
  components: {
    Sidebar,
  },
})
export default class App extends Vue {}
</script>

<style lang="scss">
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: $text;
}

.content {
  padding-left: $sidebar-width;
}
</style>
