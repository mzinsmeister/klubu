<template>
  <div class="recipent-editor">
    <b-field grouped>
      <b-field expanded label="Anrede">
        <b-input
          @input="change"
          :disabled="isDisabled"
          v-model="value.formOfAddress"
        />
      </b-field>
      <b-field expanded label="Titel">
        <b-input @input="change" :disabled="isDisabled" v-model="value.title" />
      </b-field>
    </b-field>
    <b-field label="Name">
      <b-input
        @input="change"
        :disabled="isDisabled"
        expanded
        v-model="value.firstName"
      />
      <b-input
        @input="change"
        :disabled="isDisabled"
        expanded
        v-model="value.name"
      />
    </b-field>
    <b-field grouped>
      <b-field expanded label="Straße">
        <b-input
          @input="change"
          :disabled="isDisabled"
          v-model="value.street"
        />
      </b-field>
      <b-field expanded label="Hausnummer">
        <b-input
          @input="change"
          :disabled="isDisabled"
          v-model="value.houseNumber"
        />
      </b-field>
    </b-field>
    <b-field grouped>
      <b-field expanded label="PLZ">
        <b-input
          @input="change"
          :disabled="isDisabled"
          v-model="value.zipCode"
        />
      </b-field>
      <b-field expanded label="Ort">
        <b-input @input="change" :disabled="isDisabled" v-model="value.city" />
      </b-field>
    </b-field>
    <b-field label="Land">
      <b-input @input="change" :disabled="isDisabled" v-model="value.country" />
    </b-field>
  </div>
</template>

<script lang="ts">
import { Recipient } from "@/models/CommonModel";
import { Component, Prop, Vue } from "vue-property-decorator";

@Component({
  name: "recipient-editor",
})
export default class RecipientEditor extends Vue {
  @Prop() private value!: Recipient;
  @Prop({ required: false }) private disabled?: boolean;

  private get isDisabled(): boolean {
    return this.disabled !== undefined ? this.disabled : false;
  }

  private change(): void {
    this.$emit("change");
  }
}
</script>

<style scoped lang="scss"></style>
