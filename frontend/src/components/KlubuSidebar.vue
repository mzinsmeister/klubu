<template>
  <div class="sidebar">
    <o-sidebar position="fixed" :fullheight="true" variant="light" open>
      <div class="p-1">
        <div class="block">
          <h1 class="title">KLUBU</h1>
        </div>
        <o-menu class="is-custom-mobile">
          <o-menu-list>
            <o-menu-item
              icon="view-dashboard"
              label="Dashboard"
              @click="$router.push('/')"
              :active="$route.name === 'Dashboard'"
            />
            <o-menu-item
              icon="contacts"
              label="Kontakte"
              @click="$router.push('/contacts')"
              :active="$route.name === 'Contacts'"
            />
            <o-menu-item
              icon="tag"
              label="Angebote"
              @click="$router.push('/offers')"
              :active="$route.name === 'Offers'"
            />
            <o-menu-item
              icon="file"
              label="Rechnungen"
              @click="$router.push('/invoices')"
              :active="$route.name === 'Invoices'"
            />
            <o-menu-item
              icon="receipt"
              label="Belege"
              @click="$router.push('/receipts')"
              :active="$route.name === 'Receipts'"
            />
            <o-menu-item
              icon="sigma"
              label="Auswertung"
              @click="$router.push('/reporting')"
              :active="$route.name === 'Reporting'"
            />
          </o-menu-list>
        </o-menu>
      </div>
    </o-sidebar>
  </div>
</template>

<script setup lang="ts">
</script>
<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>