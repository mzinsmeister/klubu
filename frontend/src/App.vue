<template>
  <div id="app">
    <topbar style="margin: 90px;" />
    <sidebar />
    <router-view class="content" />
  </div>
</template>
<script>
import { Component, Vue } from "vue-property-decorator";
import Topbar from "@/components/Topbar.vue";
import Sidebar from "@/components/Sidebar.vue";

@Component({
  components: {
    Topbar,
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
  padding: 20px;
}

.content {
  padding-left: $sidebar-width;
}
</style>
