<template>
    <div class="payments modal-card">
      <header class="modal-card-head">
        <p class="modal-card-title">Zahlungen</p>
        <button type="button" class="delete" @click="emit('close')" />
      </header>
      <section class="modal-card-body">
        <table
            :backend-pagination="false"
        >
        <thead>
            <th>Datum</th>
            <th>Betrag (Cents)</th>
            <th></th>
        </thead>
            <tbody>
                <tr v-for="(row, i) in pments" :key="i" class="payment-row">
                    <td><o-datepicker append-to-body v-model="row.date" @update:modelValue="updatePayment(i, {...row, date: $event})" /></td>
                    <td><o-input class="amountInput" v-model="row.amountCents" @update:modelValue="updatePayment(i, {...row, amountCents: $event !== '' ? Number.parseInt($event) : 0})" /></td>
                    <td>
                        <o-button variant="danger" icon="delete" icon-right="delete" @click="deletePayment(i)" />
                    </td>
                </tr>
            </tbody>
        </table>
      </section>
      <footer class="modal-card-foot">
        <o-button variant="primary" @click="addPayment">Zahlung hinzufügen</o-button>
      </footer>
    </div>
  </template>
  
  <script setup lang="ts">
import type { Payment } from '@/models/CommonModel';
import { onMounted, ref } from 'vue';

    const emit = defineEmits(["close", "update"]);

    // Hack: This double bookkeeping is totally stupid but
    // somehow vue doesn't seem to reactively update programatically opened
    // Modal components
    const pments = ref<Payment[]>([]);

    let props = defineProps<{
        payments: Payment[];
    }>();

    onMounted(() => {
        pments.value = props.payments;
    })

    const addPayment = () => {
        const newPayment = {
            date: new Date(),
            amountCents: 0,
        } as Payment;
        pments.value = [...pments.value, newPayment];
        emit("update", pments.value);
    };

    const updatePayment = (i: number, payment: Payment) => {
        pments.value = pments.value.map((p: Payment, index: number) => index === i ? payment : p);
        emit("update", pments.value);
    }

    const deletePayment = (i: number) => {
        pments.value = pments.value.filter((_: any, index: number) => index !== i);
        emit("update", pments.value);
    }

  </script>
  <!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss">
table {
    border-spacing: 5px;
    border-collapse: separate;
}
</style>