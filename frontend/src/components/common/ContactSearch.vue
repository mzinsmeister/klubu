<template>
  <div class="contact-search">
    <b-autocomplete
      :disabled="isDisabled"
      :data="contactSuggestions"
      v-model="contactString"
      @typing="getcontactSuggestions"
      @select="select"
      :clear-on-select="true"
    >
      <template slot-scope="props">
        <div class="customerSuggestion">
          {{ formatContact(props.option) }}
        </div>
      </template>
    </b-autocomplete>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { listContacts } from "@/services/ContactsApiService";
import { Contact } from "@/models/ContactModel";

@Component({
  name: "contact-search",
})
export default class ContactSearch extends Vue {
  @Prop() private contact!: Contact | null;
  @Prop({ required: false }) private disabled?: boolean;

  private contactSuggestions: Contact[] = [];
  private contactString = this.contact ? this.formatContact(this.contact) : "";

  private get isDisabled(): boolean {
    return this.disabled !== undefined ? this.disabled : false;
  }

  private change(): void {
    this.$emit("change");
  }

  private formatContact(contact: Contact): string {
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

  private getcontactSuggestions(name: string): void {
    listContacts(0, 10, name).then((v) => (this.contactSuggestions = v));
  }

  private select(option: Contact) {
    this.contactString = this.formatContact(option);
    this.$emit("select", option);
  }
}
</script>

<style scoped lang="scss"></style>
