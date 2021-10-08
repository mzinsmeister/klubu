<template>
  <div class="sidebar">
    <b-sidebar position="fixed" :fullheight="true" type="is-light" open>
      <div class="p-1">
        <div class="block">
          <h1 class="title">KLUBU</h1>
        </div>
        <b-menu class="is-custom-mobile">
          <b-menu-list>
            <b-menu-item
              icon="view-dashboard"
              label="Dashboard"
              @click="$router.push('/')"
              :active="$route.name === 'Dashboard'"
            />
            <b-menu-item
              icon="contacts"
              label="Kontakte"
              @click="$router.push('/contacts')"
              :active="$route.name === 'Contacts'"
            />
            <b-menu-item
              icon="tag"
              label="Angebote"
              @click="$router.push('/offers')"
              :active="$route.name === 'Offers'"
            />
            <b-menu-item
              icon="file"
              label="Rechnungen"
              @click="$router.push('/invoices')"
              :active="$route.name === 'Invoices'"
            />
            <b-menu-item
              icon="receipt"
              label="Belege"
              @click="$router.push('/receipts')"
              :active="$route.name === 'Receipts'"
            />
            <b-menu-item
              icon="sigma"
              label="Auswertung"
              @click="$router.push('/reporting')"
              :active="$route.name === 'Reporting'"
            />
          </b-menu-list>
        </b-menu>
      </div>
    </b-sidebar>
  </div>
</template>

<script lang="ts">
import { Component, Vue } from "vue-property-decorator";

@Component
export default class Sidebar extends Vue {}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>
