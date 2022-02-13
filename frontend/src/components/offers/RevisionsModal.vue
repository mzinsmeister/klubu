<template>
  <div class="revisions-modal modal-card">
    <header class="modal-card-head">
      <p class="modal-card-title">Revisionen</p>
      <button type="button" class="delete" @click="$emit('close')" />
    </header>
    <section class="modal-card-body">
      <b-table
        v-if="revisions !== null"
        :data="revisions"
        aria-next-label="Next page"
        aria-previous-label="Previous page"
        aria-page-label="Page"
        aria-current-label="Current page"
      >
        <b-table-column
          field="revisionNumber"
          label="Revision"
          width="20"
          numeric
          v-slot="props"
        >
          {{ props.row.revisionNumber }}
        </b-table-column>
        <b-table-column
          field="creationDate"
          label="Erstellt am"
          width="200"
          v-slot="props"
        >
          {{ props.row.creationDate.toLocaleDateString() }}
        </b-table-column>
        <b-table-column custom-key="actions" v-slot="props">
          <button
            class="button is-small is-light"
            @click="openRevision(props.row.revisionNumber)"
          >
            Öffnen
          </button>
        </b-table-column>
      </b-table>
    </section>
    <footer class="modal-card-foot">
      <b-button @click="createRevision">Neue Revision</b-button>
    </footer>
  </div>
</template>

<script lang="ts">
import { OfferRevision } from "@/models/OfferModel";
import { getOfferRevisions } from "@/services/OffersApiService";
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class RevisionsModal extends Vue {
  @Prop() private offerId!: number;
  private revisions: Array<OfferRevision> | null = null;

  private created() {
    getOfferRevisions(this.offerId).then((r) => {
      this.revisions = r.sort(
        (a, b) => b.creationDate.getTime() - a.creationDate.getTime()
      );
    });
  }

  private createRevision() {
    this.$emit("createRevision");
    this.$emit("close");
  }

  private openRevision(revisionNumber: number) {
    this.$router.push(`/offers/${this.offerId}/revisions/${revisionNumber}`);
    this.$emit("close");
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>
