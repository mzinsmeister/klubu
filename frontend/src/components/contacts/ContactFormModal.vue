<template>
  <div class="contact-form modal-card">
    <header class="modal-card-head">
      <p class="modal-card-title">Neuen Kontakt erstellen</p>
      <button type="button" class="delete" @click="$emit('close')" />
    </header>
    <section class="modal-card-body">
      <b-field label="Name*">
        <b-input
          v-model="contact.name"
          placeholder="Name(Firma) oder Nachname(Privat)"
          required
        >
        </b-input>
      </b-field>

      <b-field label="Titel">
        <b-input v-model="contact.title" placeholder="Titel"></b-input>
      </b-field>

      <b-field label="Anrede">
        <b-input v-model="contact.formOfAddress" placeholder="Anrede"></b-input>
      </b-field>

      <b-field label="Vorname">
        <b-input v-model="contact.firstName" placeholder="Vorname"></b-input>
      </b-field>

      <b-field label="Straße">
        <b-input v-model="contact.street" placeholder="Straße"></b-input>
      </b-field>

      <b-field label="Hausnummer">
        <b-input
          v-model="contact.houseNumber"
          placeholder="Hausnummer"
        ></b-input>
      </b-field>

      <b-field label="Postleitzahl">
        <b-input v-model="contact.zipCode" placeholder="Postleitzahl"></b-input>
      </b-field>

      <b-field label="Stadt">
        <b-input v-model="contact.city" placeholder="Stadt"></b-input>
      </b-field>

      <b-field label="Land">
        <b-input v-model="contact.country" placeholder="Land"></b-input>
      </b-field>

      <b-field label="Telefonnummer">
        <b-input v-model="contact.phone" placeholder="Telefonnummer"></b-input>
      </b-field>

      <b-field label="Person?">
        <b-switch v-model="contact.isPerson" />
      </b-field>
    </section>
    <footer class="modal-card-foot">
      <b-button
        label="Erstellen"
        type="is-primary"
        :disabled="contact.name.length === 0"
        @click="create"
        :loading="creating"
      />
    </footer>
  </div>
</template>

<script lang="ts">
import { createContact } from "@/services/ContactsApiService";
import { Component, Vue } from "vue-property-decorator";

@Component
export default class ContactFormModal extends Vue {
  private contact = {
    name: "",
    country: "Deutschland",
    isPerson: false,
  };
  private creating = false;

  create(): void {
    createContact(this.contact).then(() => {
      this.$emit("newContact");
      this.$emit("close");
    });
    this.creating = true;
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>
