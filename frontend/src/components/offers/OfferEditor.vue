<template>
  <div class="offer-editor" v-if="offer !== null">
    <div>
      <div class="top-buttons">
        <b-button type="is-info" @click="back">Zurück</b-button>
        <b-button type="is-success" @click="save">Speichern</b-button>
        <b-button
          type="is-warning"
          :loading="isExporting"
          :disabled="offer.id === undefined"
          @click="tryExport"
          >Exportieren</b-button
        >
      </div>
    </div>
    <b-field label="Titel">
      <b-input @input="change" v-model="offer.title" />
    </b-field>
    <b-field label="Kunde">
      <contact-search
        :contact="
          offer.customerContact === undefined ? null : offer.customerContact
        "
        @select="select"
      />
    </b-field>
    <recipient-editor @change="change" v-model="offer.recipient" />
    <b-field grouped>
      <b-field expanded label="Angebotsdatum">
        <b-datepicker @input="change" v-model="offer.offerDate" />
        <p class="control">
          <b-button
            @click="
              offer.offerDate = undefined;
              change();
            "
            icon-right="delete"
            :disabled="offer.offerDate === undefined"
          />
        </p>
      </b-field>
      <b-field expanded label="Gültig bis">
        <b-datepicker @input="change" v-model="offer.validUntilDate" />
        <p class="control">
          <b-button
            @click="
              offer.validUntilDate = undefined;
              change();
            "
            icon-right="delete"
            :disabled="offer.validUntilDate === undefined"
          />
        </p>
      </b-field>
    </b-field>
    <b-field label="Betreff">
      <b-input @input="change" v-model="offer.subject" />
    </b-field>
    <b-field label="Einleitungstext">
      <b-input @input="change" type="textarea" v-model="offer.headerHTML" />
    </b-field>
    <items-editor @change="change" v-model="offer.items" />
    <b-field label="Fußtext">
      <b-input @input="change" type="textarea" v-model="offer.footerHTML" />
    </b-field>
    <p>Gesamt: {{ getTotal() }}</p>
  </div>
</template>

<script lang="ts">
import { Contact } from "@/models/ContactModel";
import { Offer } from "@/models/OfferModel";
import RecipientEditor from "../common/RecipientEditor.vue";
import ContactSearch from "../common/ContactSearch.vue";
import ItemsEditor from "../common/ItemsEditor.vue";
import {
  createOffer,
  exportOffer,
  fetchOffer,
  updateOffer,
} from "@/services/OffersApiService";
import { Component, Vue } from "vue-property-decorator";
import { formatCentsAsMoney } from "@/util/MoneyUtil";

@Component({
  name: "offer-editor",
  components: {
    RecipientEditor,
    ContactSearch,
    ItemsEditor,
  },
})
export default class OfferEditor extends Vue {
  private offer: Offer | null = null;
  private changed = false; //TODO: Warn if exporting with unsaved changes
  private changedSinceSave = false;

  private isExporting = false;

  private change() {
    this.changedSinceSave = true;
  }

  private getTotal(): string {
    let total = 0;
    if (this.offer !== null) {
      this.offer.items.forEach((item) => {
        total += Number.parseInt(
          (item.price.amountCents * item.quantity).toFixed(0)
        );
      });
    }
    return this.formatCentsAsMoney(total);
  }

  private tryExport() {
    if (this.changedSinceSave) {
      this.$buefy.dialog.confirm({
        message:
          "Das Angebot enthält ungespeicherte Änderungen!\n" +
          "Trotzdem exportieren (ohne ungespeicherte Änderungen)?",
        title: "Ungespeicherte Änderungen",
        onConfirm: this.export,
        trapFocus: true,
        canCancel: true,
        confirmText: "Ja",
        cancelText: "Abbrechen",
      });
    } else {
      this.export();
    }
  }

  private export() {
    if (this.offer !== null) {
      exportOffer(this.offer)
        .then(() => {
          this.isExporting = false;
          this.$buefy.toast.open({
            message: "Export erfolgreich",
            type: "is-success",
          });
        })
        .catch(() => {
          this.isExporting = false;
          this.$buefy.toast.open({
            message: "Fehler beim Export",
            type: "is-danger",
          });
        });
      this.isExporting = true;
    }
  }

  private formatCentsAsMoney(cents: number): string {
    return formatCentsAsMoney(cents);
  }

  private back() {
    this.$router.push({
      path: "/offers",
      query: { forceRefresh: this.changed.toString() },
    });
  }

  private select(option: Contact) {
    if (this.offer !== null) {
      this.offer.customerContact = option;
      this.offer.recipient!.formOfAddress = option.formOfAddress;
      this.offer.recipient!.title = option.title;
      this.offer.recipient!.name = option.name;
      this.offer.recipient!.firstName = option.firstName;
      this.offer.recipient!.street = option.street;
      this.offer.recipient!.zipCode = option.zipCode;
      this.offer.recipient!.city = option.city;
      this.offer.recipient!.houseNumber = option.houseNumber;
      this.offer.recipient!.country = option.country;
    }
    this.change();
  }

  private save(): void {
    if (this.offer !== null && this.offer?.id === undefined) {
      createOffer(this.offer).then((result) => {
        history.replaceState(
          history.state,
          document.title,
          "/offers/" + result.id
        );
        this.offer = result;
        this.changedSinceSave = false;
      });
    } else if (this.offer !== null) {
      updateOffer(this.offer).then(() => (this.changedSinceSave = false));
    }
    this.changed = true;
  }

  private created(): void {
    const id = this.$route.params["id"];
    if (id === "new") {
      this.offer = { items: [], subject: "Angebot", recipient: { name: "" } };
    } else {
      fetchOffer(Number.parseInt(id)).then((v) => {
        if (v.recipient === undefined) {
          v.recipient = { name: "" };
        }
        this.offer = v;
      });
    }
  }
}
</script>

<style scoped lang="scss">
.position-input {
  margin-left: auto;
  margin-right: auto;
}
.top-buttons {
  display: flex;
  justify-content: space-between;
  padding: 10px;
}
</style>
