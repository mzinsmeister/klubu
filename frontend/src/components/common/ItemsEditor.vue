<template>
  <div class="items-editor">
    <table>
      <thead>
        <th></th>
        <th>Position</th>
        <th>Menge</th>
        <th>Einheit</th>
        <th>Einzelpreis (Cent)</th>
        <th>Betrag</th>
        <th></th>
      </thead>
      <tbody>
        <tr v-for="(item, i) in value" :key="i">
          <td :value="i + 1" />
          <td>
            <b-input
              @input="change"
              :disabled="isDisabled"
              class="position-input"
              v-model="item.item"
            />
          </td>
          <td>
            <b-input
              @input="change"
              :disabled="isDisabled"
              class="position-input"
              style="max-width: 50px"
              v-model="item.quantity"
            />
          </td>
          <td>
            <b-input
              @input="change"
              :disabled="isDisabled"
              class="position-input"
              style="max-width: 80px"
              v-model="item.unit"
            />
          </td>
          <td>
            <b-input
              @input="change"
              :disabled="isDisabled"
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
              :disabled="isDisabled"
              icon-right="delete"
              type="is-danger"
              @click="deleteItem(i)"
            />
          </td>
        </tr>
      </tbody>
    </table>
    <b-button @click="addEmptyItem()" :disabled="isDisabled"
      >Zusätzliche Position</b-button
    >
  </div>
</template>

<script lang="ts">
import { Item } from "@/models/CommonModel";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { Component, Prop, Vue } from "vue-property-decorator";

@Component({
  name: "items-editor",
})
export default class ItemsEditor extends Vue {
  @Prop() private value!: Item[];
  @Prop({ required: false }) private disabled?: boolean;

  private created() {
    if (this.value.length === 0) {
      this.addEmptyItem();
    }
  }

  private get isDisabled(): boolean {
    return this.disabled !== undefined ? this.disabled : false;
  }

  private deleteItem(index: number) {
    this.value.splice(index, 1);
    this.change();
  }

  private formatCentsAsMoney(cents: number): string {
    return formatCentsAsMoney(cents);
  }

  private addEmptyItem(): void {
    const newItem: Item = {
      item: "",
      quantity: 1,
      unit: "",
      price: { amountCents: 0, currency: { code: "EUR" } },
    };
    this.value.push(newItem);
    this.change();
  }

  private change(): void {
    this.$emit("change");
  }
}
</script>

<style scoped lang="scss"></style>
