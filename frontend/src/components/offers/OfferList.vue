<template>
  <div class="offer-list" >
    <b-table
      v-if="offers !== null"
      :data="offers"
      :backend-pagination="true"
      aria-next-label="Next page"
      aria-previous-label="Previous page"
      aria-page-label="Page"
      aria-current-label="Current page"
    >
      <b-table-column field="id" label="ID" width="20" numeric v-slot="props">
        {{ props.row.id }}
      </b-table-column>
      <b-table-column field="title" label="Titel" width="200" v-slot="props">
        {{ props.row.title }}
      </b-table-column>
      <b-table-column
        field="customerContact.name"
        label="Kunde"
        width="200"
        v-slot="props"
      >
        {{ getCustomerName(props.row.customerContact) }}
      </b-table-column>
      <b-table-column custom-key="actions" v-slot="props">
        <button class="button is-small is-light" @click="view(props.row.id)">
          Öffnen
        </button>
      </b-table-column>
    </b-table>
  </div>
</template>

<script lang="ts">
import { Contact } from "@/models/ContactModel";
import { OfferListItem } from "@/models/OfferModel";
import { listOffers } from "@/services/OffersApiService";
import { Component, Vue } from "vue-property-decorator";

const PAGE_SIZE = 100000;

@Component({
  name: "offer-list",
})
export default class OfferList extends Vue {
  private pagesCache: Map<number, Array<OfferListItem>> = new Map();
  private offers: Array<OfferListItem> | null = null;

  private created(): void {
    this.pageChange(0);
  }

  private activated(): void {
    console.log(this.$route.params);
    if (this.$route.query["forceRefresh"] === "true") {
      this.clearCache();
      this.reload();
    }
  }

  private view(id: number): void {
    this.$router.push(`/offers/${id}`);
  }

  private getCustomerName(customerContact: Contact | undefined): string {
    return customerContact?.name ?? "";
  }

  clearCache(): void {
    this.pagesCache = new Map();
  }

  reload(): void {
    this.pageChange(0);
  }

  private pageChange(page: number): void {
    this.offers = this.pagesCache.get(page) ?? null;
    if (this.offers === null) {
      listOffers(page, PAGE_SIZE).then((v) => {
        this.offers = v;
        this.pagesCache.set(page, v);
      });
    }
  }
}
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
