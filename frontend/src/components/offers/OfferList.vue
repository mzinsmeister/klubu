<template>
  <div class="offer-list">
    <o-table
      v-if="offers !== null"
      :data="offers"
      :backend-pagination="true"
      aria-next-label="Next page"
      aria-previous-label="Previous page"
      aria-page-label="Page"
      aria-current-label="Current page"
    >
      <o-table-column
        field="id"
        label="Angebotsnr."
        width="20"
        numeric
        v-slot="props"
      >
        {{ props.row.id }}
      </o-table-column>
      <o-table-column field="title" label="Titel" width="200" v-slot="props">
        {{ props.row.title }}
      </o-table-column>
      <o-table-column
        field="customerContact.name"
        label="Kunde"
        width="200"
        v-slot="props"
      >
        {{ getCustomerName(props.row.customerContact) }}
      </o-table-column>
      <o-table-column custom-key="actions" v-slot="props">
        <button class="button is-small is-light" @click="view(props.row.id)">
          Öffnen
        </button>
      </o-table-column>
    </o-table>
  </div>
</template>

<script setup lang="ts">

import { onMounted, ref, type Ref } from "vue";
import { type Contact } from "@/models/ContactModel";
import { listOffers } from "@/services/OffersApiService";
import { type OfferListItem } from "@/models/OfferModel";
import { useRoute, useRouter } from "vue-router";



const route = useRoute();
const router = useRouter();
const PAGE_SIZE = 100000;
let pagesCache: Map<number, Array<OfferListItem>> = new Map();
const offers: Ref<Array<OfferListItem> | null> = ref(null);

const view = (id: number): void => {
  router.push(`/offers/${id}`);
}
const getCustomerName = (customerContact: Contact | undefined): string => {
  return customerContact?.name ?? "";
}
const clearCache = (): void => {
  pagesCache = new Map();
}
const reload = (): void => {
  pageChange(0);
}
if (route.query["forceRefresh"] === "true") {
  clearCache();
  reload();
}
const pageChange = (page: number): void => {
  offers.value = pagesCache.get(page) ?? null;
  if (offers.value === null) {
    listOffers(page, PAGE_SIZE).then((v) => {
      offers.value = v;
      pagesCache.set(page, v);
    });
  }
}

onMounted(() =>{
  pageChange(0);
});
</script>
<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss">
h3 {
  margin: 40px 0 0;
}
ul {
  list-style-type: none;
  padding: 0;
}
li {
  display: inline-block;
  margin: 0 10px;
}
a {
  color: $text-invert;
}
</style>