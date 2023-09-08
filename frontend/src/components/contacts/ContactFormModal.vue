<template>
  <div class="contact-form modal-card">
    <header class="modal-card-head">
      <p class="modal-card-title">Neuen Kontakt erstellen</p>
      <button type="button" class="delete" @click="emit('close')" />
    </header>
    <section class="modal-card-body">
      <o-field label="Name*">
        <o-input
          v-model="editedContact.name"
          placeholder="Name(Firma) oder Nachname(Privat)"
          required
        >
        </o-input>
      </o-field>

      <o-field label="Titel">
        <o-input v-model="editedContact.title" placeholder="Titel"></o-input>
      </o-field>

      <o-field label="Anrede">
        <o-input
          v-model="editedContact.formOfAddress"
          placeholder="Anrede"
        ></o-input>
      </o-field>

      <o-field label="Vorname">
        <o-input
          v-model="editedContact.firstName"
          placeholder="Vorname"
        ></o-input>
      </o-field>

      <o-field label="Straße">
        <o-input v-model="editedContact.street" placeholder="Straße"></o-input>
      </o-field>

      <o-field label="Hausnummer">
        <o-input
          v-model="editedContact.houseNumber"
          placeholder="Hausnummer"
        ></o-input>
      </o-field>

      <o-field label="Postleitzahl">
        <o-input
          v-model="editedContact.zipCode"
          placeholder="Postleitzahl"
        ></o-input>
      </o-field>

      <o-field label="Stadt">
        <o-input v-model="editedContact.city" placeholder="Stadt"></o-input>
      </o-field>

      <o-field label="Land">
        <o-input v-model="editedContact.country" placeholder="Land"></o-input>
      </o-field>

      <o-field label="Telefonnummer">
        <o-input
          v-model="editedContact.phone"
          placeholder="Telefonnummer"
        ></o-input>
      </o-field>

      <o-field label="Person?">
        <o-switch v-model="editedContact.isPerson" />
      </o-field>
    </section>
    <footer class="modal-card-foot">
      <o-button
        :label="isNew ? 'Erstellen' : 'Speichern'"
        type="is-primary"
        :disabled="editedContact.name.length === 0"
        @click="save"
        :loading="saving"
      />
    </footer>
  </div>
</template>

<script setup lang="ts">

import { ref, computed } from "vue";
import { type Contact } from "@/models/ContactModel";
import { createContact, updateContact } from "@/services/ContactsApiService";



const emit = defineEmits(["change", "close"]);
const { contact } = defineProps<{ contact?: Contact }>();

const getEditedContact = (): Contact => {
  if (contact === undefined) {
    return {
      name: "",
      country: "Deutschland",
      isPerson: false,
    };
  } else {
    return JSON.parse(JSON.stringify(contact));
  }
}

const editedContact = ref<Contact>(getEditedContact());
const saving = ref(false);

const isNew = computed((): boolean => {
  return editedContact.value.id === undefined;
});

const save = (): void => {
  if (editedContact.value.id === undefined) {
    createContact(editedContact.value).then(() => {
      emit("change");
      emit("close");
    });
  } else {
    updateContact(editedContact.value).then(() => {
      emit("change");
      emit("close");
    });
  }

  saving.value = true;
}
</script>
<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>