<template>
  <div class="revisions-modal modal-card">
    <header class="modal-card-head">
      <p class="modal-card-title">Revisionen</p>
      <button type="button" class="delete" @click="emit('close')" />
    </header>
    <section class="modal-card-body">
      <o-table
        v-if="revisions !== null"
        :data="revisions"
        aria-next-label="Next page"
        aria-previous-label="Previous page"
        aria-page-label="Page"
        aria-current-label="Current page"
      >
        <o-table-column
          field="revisionNumber"
          label="Revision"
          width="20"
          numeric
          v-slot="props"
        >
          {{ props.row.revisionNumber }}
        </o-table-column>
        <o-table-column
          field="creationDate"
          label="Erstellt am"
          width="200"
          v-slot="props"
        >
          {{ props.row.creationDate.toLocaleDateString() }}
        </o-table-column>
        <o-table-column custom-key="actions" v-slot="props">
          <button
            class="button is-small is-light"
            @click="openRevision(props.row.revisionNumber)"
          >
            Öffnen
          </button>
        </o-table-column>
      </o-table>
    </section>
    <footer class="modal-card-foot">
      <o-button @click="createRevision">Neue Revision</o-button>
    </footer>
  </div>
</template>

<script setup lang="ts">

import { ref, type Ref } from "vue";
import { getOfferRevisions } from "@/services/OffersApiService";
import { type OfferRevision } from "@/models/OfferModel";
import { useRouter } from "vue-router";



const { offerId } = defineProps<{
    offerId:  number, 
  }>()

const emit = defineEmits(["createRevision", "close"]);


  const router = useRouter();
  const revisions: Ref<Array<OfferRevision> | null> = ref(null);
  getOfferRevisions(offerId).then((r) => {
    revisions.value = r.sort(
      (a, b) => b.creationDate.getTime() - a.creationDate.getTime()
    );
  });
  const createRevision = ()  => {
    emit("createRevision");
    emit("close");
  }
  const openRevision = (revisionNumber: number)  => {
    router.push(`/offers/${offerId}/revisions/${revisionNumber}`);
    emit("close");
  }
</script>
<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>