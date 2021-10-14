<template>
  <div class="contact-form modal-card">
    <header class="modal-card-head">
      <p class="modal-card-title">Neuen Kontakt erstellen</p>
      <button type="button" class="delete" @click="$emit('close')" />
    </header>
    <section class="modal-card-body">
      <b-field label="Name*">
        <b-input
          v-model="editedContact.name"
          placeholder="Name(Firma) oder Nachname(Privat)"
          required
        >
        </b-input>
      </b-field>

      <b-field label="Titel">
        <b-input v-model="editedContact.title" placeholder="Titel"></b-input>
      </b-field>

      <b-field label="Anrede">
        <b-input
          v-model="editedContact.formOfAddress"
          placeholder="Anrede"
        ></b-input>
      </b-field>

      <b-field label="Vorname">
        <b-input
          v-model="editedContact.firstName"
          placeholder="Vorname"
        ></b-input>
      </b-field>

      <b-field label="Straße">
        <b-input v-model="editedContact.street" placeholder="Straße"></b-input>
      </b-field>

      <b-field label="Hausnummer">
        <b-input
          v-model="editedContact.houseNumber"
          placeholder="Hausnummer"
        ></b-input>
      </b-field>

      <b-field label="Postleitzahl">
        <b-input
          v-model="editedContact.zipCode"
          placeholder="Postleitzahl"
        ></b-input>
      </b-field>

      <b-field label="Stadt">
        <b-input v-model="editedContact.city" placeholder="Stadt"></b-input>
      </b-field>

      <b-field label="Land">
        <b-input v-model="editedContact.country" placeholder="Land"></b-input>
      </b-field>

      <b-field label="Telefonnummer">
        <b-input
          v-model="editedContact.phone"
          placeholder="Telefonnummer"
        ></b-input>
      </b-field>

      <b-field label="Person?">
        <b-switch v-model="editedContact.isPerson" />
      </b-field>
    </section>
    <footer class="modal-card-foot">
      <b-button
        :label="isNew ? 'Erstellen' : 'Speichern'"
        type="is-primary"
        :disabled="editedContact.name.length === 0"
        @click="save"
        :loading="saving"
      />
    </footer>
  </div>
</template>

<script lang="ts">
import { Contact } from "@/models/ContactModel";
import { createContact, updateContact } from "@/services/ContactsApiService";
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class ContactFormModal extends Vue {
  @Prop({ required: false }) private contact?: Contact;

  private editedContact: Contact = this.getEditedContact();
  private saving = false;

  private get isNew(): boolean {
    return this.editedContact.id === undefined;
  }

  private getEditedContact(): Contact {
    if (this.contact === undefined) {
      return {
        name: "",
        country: "Deutschland",
        isPerson: false,
      };
    } else {
      return JSON.parse(JSON.stringify(this.contact));
    }
  }

  save(): void {
    if (this.editedContact.id === undefined) {
      createContact(this.editedContact).then(() => {
        this.$emit("change");
        this.$emit("close");
      });
    } else {
      updateContact(this.editedContact).then(() => {
        this.$emit("change");
        this.$emit("close");
      });
    }
    this.saving = true;
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>
