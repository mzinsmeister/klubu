import Vue from "vue";
import VueRouter, { RouteConfig } from "vue-router";
import Contacts from "../views/Contacts.vue";
import Home from "../views/Home.vue";
import Invoices from "../views/Invoices.vue";
import InvoicesOverview from "../components/invoices/InvoicesOverview.vue";
import InvoiceEditor from "../components/invoices/InvoiceEditor.vue";
import Offers from "../views/Offers.vue";
import OffersOverview from "../components/offers/OffersOverview.vue";
import OfferEditor from "../components/offers/OfferEditor.vue";
import Receipts from "../views/Receipts.vue";
import ReceiptsOverview from "../components/receipts/ReceiptsOverview.vue";
import ReceiptEditor from "../components/receipts/ReceiptEditor.vue";
import Reporting from "../views/Reporting.vue";

Vue.use(VueRouter);

const routes: Array<RouteConfig> = [
  {
    path: "/",
    name: "Dashboard",
    component: Home,
  },
  {
    path: "/contacts",
    name: "Contacts",
    component: Contacts,
  },
  {
    path: "/offers",
    children: [
      {
        path: ":id",
        name: "OfferEditor",
        component: OfferEditor,
      },
      {
        path: ":id/revisions/:revision",
        name: "OfferEditorWithRevision",
        component: OfferEditor,
      },
      {
        path: "",
        name: "Offers",
        component: OffersOverview,
      },
    ],
    component: Offers,
  },
  {
    path: "/invoices",
    children: [
      {
        path: ":id",
        name: "InvoiceEditor",
        component: InvoiceEditor,
      },
      {
        path: "",
        name: "Invoices",
        component: InvoicesOverview,
      },
    ],
    component: Invoices,
  },
  {
    path: "/receipts",
    children: [
      {
        path: ":id",
        name: "ReceiptEditor",
        component: ReceiptEditor,
      },
      {
        path: "",
        name: "Receipts",
        component: ReceiptsOverview,
      },
    ],
    component: Receipts,
  },
  {
    path: "/reporting",
    name: "Reporting",
    component: Reporting,
  },
];

const router = new VueRouter({
  mode: "history",
  base: process.env.BASE_URL,
  routes,
});

export default router;
