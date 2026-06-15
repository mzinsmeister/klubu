<template>
  <div class="props.contact-search">
    <o-autocomplete
      :disabled="isDisabled"
      :options="contactSuggestions"
      v-model:input="contactString"
      @typing="getcontactSuggestions"
      @select="select"
      :clear-on-select="true"
    >
      <template #option="{ option }">
        <div class="customerSuggestion">
          {{ formatContact(toContact(option.item)) }}
        </div>
      </template>
    </o-autocomplete>
  </div>
</template>

<script setup lang="ts">
import { OAutocomplete } from "@oruga-ui/oruga-next"
import { ref, computed } from "vue";
import type { Contact } from "@/models/ContactModel";
import { listContacts } from "@/services/ContactsApiService";

const formatContact = (contact: Contact): string => {
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

const props = defineProps<{
  contact?:  Contact | null, 
  disabled?: boolean
}>();

const emit = defineEmits(["select", "change"]);

const contactSuggestions = ref<Contact[]>([]);
let contactString = props.contact ? formatContact(props.contact) : "";
const isDisabled = computed((): boolean => {
  return props.disabled !== undefined ? props.disabled : false;
});

const toContact = (val: any): Contact => val;

const getcontactSuggestions = (name: string): void => {
  listContacts(0, 10, name).then((v) => (contactSuggestions.value = v));
}
const select = (option: any)  => {
  const contact = toContact(option);
  contactString = formatContact(contact);
  emit("select", contact);
}
</script>
<style scoped lang="scss"></style>