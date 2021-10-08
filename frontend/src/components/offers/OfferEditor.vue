<template>
  <div class="offer-editor" v-if="offer !== null">
    <b-button type="is-info" @click="back">Zurück</b-button>
    <b-button type="is-success" outlined @click="save">Speichern</b-button>
    <b-button type="is-danger" outlined @click="exportThis">Exportieren</b-button>
    <b-field label="Titel">
      <b-input v-model="offer.title" />
    </b-field>
    <b-field label="Kunde">
      <b-autocomplete
        :data="customerSuggestions"
        v-model="customerString"
        @typing="getCustomerSuggestions"
        @select="select"
        :clear-on-select="true"
      >
        <template slot-scope="props">
          <div class="customerSuggestion">
            {{ formatCustomer(props.option) }}
          </div>
        </template>
      </b-autocomplete>
    </b-field>
    <b-field grouped>
      <b-field expanded label="Anrede">
        <b-input v-model="offer.recipent.formOfAddress" />
      </b-field>
      <b-field expanded label="Titel">
        <b-input v-model="offer.recipent.title" />
      </b-field>
    </b-field>
    <b-field label="Name">
      <b-input expanded v-model="offer.recipent.firstName" />
      <b-input expanded v-model="offer.recipent.name" />
    </b-field>
    <b-field grouped>
      <b-field expanded label="Straße">
        <b-input v-model="offer.recipent.street" />
      </b-field>
      <b-field expanded label="Hausnummer">
        <b-input v-model="offer.recipent.houseNumber" />
      </b-field>
    </b-field>
    <b-field grouped>
      <b-field expanded label="PLZ">
        <b-input v-model="offer.recipent.zipCode" />
      </b-field>
      <b-field expanded label="Ort">
        <b-input v-model="offer.recipent.city" />
      </b-field>
    </b-field>
    <b-field label="Land">
      <b-input v-model="offer.recipent.country" />
    </b-field>
    <b-field label="Einleitungstext">
      <b-input type="textarea" v-model="offer.headerHTML" />
    </b-field>
    <table>
      <thead>
        <th></th>
        <th>Position</th>
        <th>Menge</th>
        <th>Einheit</th>
        <th>Einzelpreis</th>
        <th>Betrag</th>
      </thead>
      <tbody>
        <tr v-for="(item, i) in offer.items" :key="i">
          <td :value="i + 1" />
          <td>
            <b-input class="position-input" v-model="item.item" />
          </td>
          <td>
            <b-input
              class="position-input"
              style="max-width: 50px"
              v-model="item.quantity"
            />
          </td>
          <td>
            <b-input
              class="position-input"
              style="max-width: 80px"
              v-model="item.unit"
            />
          </td>
          <td>
            <b-input
              class="position-input"
              style="max-width: 100px"
              v-model="item.price.amountCents"
            />
          </td>
          <td>
            {{ formatCentsAsMoney(item.quantity * item.price.amountCents) }}
          </td>
          <td>
            <b-button
              icon-right="delete"
              type="is-danger"
              @click="deleteItem(i)"
            />
          </td>
        </tr>
      </tbody>
    </table>
    <b-button @click="addEmptyItem()">Zusätzliche Position</b-button>
    <b-field label="Fußtext">
      <b-input type="textarea" v-model="offer.footerHTML" />
    </b-field>
    <p>Gesamt: {{ getTotal() }}</p>
  </div>
</template>

<script lang="ts">
import { Contact } from "@/models/ContactModel";
import { Offer, OfferItem } from "@/models/OfferModel";
import { listContacts } from "@/services/ContactsApiService";
import {
  createOffer,
  fetchOffer,
  updateOffer,
} from "@/services/OffersApiService";
import { Component, Vue } from "vue-property-decorator";

@Component({
  name: "offer-editor",
})
export default class OfferEditor extends Vue {
  private offer: Offer | null = null;
  private changed = false;

  private customerSuggestions: Contact[] = [];

  private customerString = "";

  private getCustomerSuggestions(name: string): void {
    listContacts(0, 10, name).then((v) => (this.customerSuggestions = v));
  }

  private deleteItem(index: number) {
    this.offer?.items.splice(index, 1);
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

  private exportThis() {
    console.log("export");
  }

  private formatCentsAsMoney(cents: number): string {
    let string = cents.toString().padStart(3, "0");
    return (
      string.substring(0, string.length - 2) +
      "," +
      string.substring(string.length - 2)
    );
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
      this.offer.recipent!.formOfAddress = option.formOfAddress;
      this.offer.recipent!.title = option.title;
      this.offer.recipent!.name = option.name;
      this.offer.recipent!.firstName = option.firstName;
      this.offer.recipent!.street = option.street;
      this.offer.recipent!.zipCode = option.zipCode;
      this.offer.recipent!.city = option.city;
      this.offer.recipent!.houseNumber = option.houseNumber;
      this.offer.recipent!.country = option.country;
    }
    this.customerString = this.formatCustomer(option);
  }

  private formatCustomer(contact: Contact): string {
    let result = contact.name;
    if (
      contact.firstName !== undefined &&
      contact.firstName !== null &&
      contact.firstName.length > 0
    ) {
      result += ", " + contact.firstName;
    }
    return result;
  }

  private prefillRecipient(): void {
    if (
      this.offer !== null &&
      (this.offer.recipent == undefined || this.offer.recipent == null)
    ) {
      this.offer.recipent = {
        name: "",
      };
    }
  }

  private addEmptyItem(): void {
    const newItem: OfferItem = {
      item: "",
      quantity: 1,
      unit: "",
      price: { amountCents: 0, currency: { code: "EUR" } },
    };
    this.offer?.items.push(newItem);
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
      });
    } else if (this.offer !== null) {
      updateOffer(this.offer);
    }
    this.changed = true;
  }

  private created(): void {
    const id = this.$route.params["id"];
    if (id === "new") {
      this.offer = { items: [] };
      this.prefillRecipient();
      this.addEmptyItem();
    } else {
      fetchOffer(Number.parseInt(id)).then((v) => {
        this.offer = v;
        this.prefillRecipient();
        if (v.customerContact !== undefined && v.customerContact !== null) {
          this.customerString = this.formatCustomer(v.customerContact);
        }
        if (v.items.length === 0) {
          this.addEmptyItem();
        }
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
</style>
