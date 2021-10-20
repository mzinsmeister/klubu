<template>
	<div class="Topbar">
		<b-sidebar
			background="floralwhite"
			position="fixed"
			:fullheight="true"
			type="is-light"
			open
		>
			<div
				class="navbar-right-container"
				style="position: inherit;display: block;height: 82px;width:100%"
			>
				<ul
					class="nav navbar-nav navbar-right"
					style="display: flex;justify-content: right;height: 100%;padding: 18px;"
				>
					<li class="nav navbar-nav navbar-form global-search-container">
						<div class="input-group has-feedback">
							<input
								type="text"
								class="form-control global-search-input"
								placeholder="Suchen"
								autocomplete="espo-global-search"
								style="float: none;display: block;text-align: left;width: 100%;margin: 0;padding: 14px;border: 1px solid #ccc;"
							/>
							<div class="input-group-btn">
								<a
									class="btn btn-link global-search-button"
									data-action="search"
									title="Suchen"
									><span class="fas fa-search"></span
								></a>
							</div>
						</div>
						<div class="global-search-panel-container"></div>
					</li>

					<li class="dropdown notifications-badge-container">
						<a
							href="javascript:"
							class="notifications-button"
							data-action="showNotifications"
							title="Benachrichtigungen"
							style="font-size: x-large;padding: 7px;"
						>
							<i class="mdi mdi-magnify"></i>
							<span class="badge number-badge hidden"></span>
						</a>
						<div class="notifications-panel-container"></div>
					</li>
					<li
						class="dropdown notifications-badge-container"
						style="font-size: x-large;padding: 7px;"
					>
						<a
							href="javascript:"
							class="notifications-button"
							data-action="showNotifications"
							title="Benachrichtigungen"
						>
							<i class="mdi mdi-bell"></i>
							<span class="badge number-badge hidden"></span>
						</a>
            <ul class="dropdown-menu" role="menu" aria-labelledby="nav-quick-create-dropdown">
                    <li class="dropdown-header">Notifications</li>
                    <li><a href="#Account/create" data-name="Account" data-action="quick-create">None</a></li>
                </ul>
					</li>
					<li
						class="dropdown menu-container"
						style="font-size: x-large;padding: 7px;"
					>
						<a
							id="nav-menu-dropdown"
							class="dropdown-toggle"
							data-toggle="dropdown"
							href="#"
							title="Menü"
							aria-expanded="false"
							><i class="mdi mdi-plus"></i
						></a>
            <ul class="dropdown-menu" role="menu" aria-labelledby="nav-quick-create-dropdown">
                    <li class="dropdown-header">Erstellen</li>
                    <li><a href="#Account/create" data-name="Account" data-action="quick-create">Firma</a></li>
                    <li><a href="#Contact/create" data-name="Contact" data-action="quick-create">Kontakt</a></li>
                    <li><a href="#Lead/create" data-name="Lead" data-action="quick-create">Interessent</a></li>
                    <li><a href="#Opportunity/create" data-name="Opportunity" data-action="quick-create">Verkaufschance</a></li>
                    <li><a href="#Meeting/create" data-name="Meeting" data-action="quick-create">Meeting</a></li>
                    <li><a href="#Call/create" data-name="Call" data-action="quick-create">Anruf</a></li>
                    <li><a href="#Task/create" data-name="Task" data-action="quick-create">Aufgabe</a></li>
                    <li><a href="#Case/create" data-name="Case" data-action="quick-create">Fall</a></li>
                    <li><a href="#Email/create" data-name="Email" data-action="quick-create">E-Mail</a></li>
                </ul>
					</li>
					<li
						class="dropdown menu-container"
						style="font-size: x-large;padding: 7px;"
					>
						<a
							id="nav-menu-dropdown"
							class="dropdown-toggle"
							data-toggle="dropdown"
							href="#"
							title="Menü"
							aria-expanded="false"
							><i class="mdi mdi-dots-vertical"></i
						></a>
            <ul class="dropdown-menu" role="menu" aria-labelledby="nav-menu-dropdown">
          <li>
            <a href="#User/view/1" class="nav-link"
              >
              User</a
            >
          </li>
          <li class="divider"></li>
          <li><a href="#Admin" class="nav-link">Administration</a></li>
          <li>
            <a href="#Preferences" class="nav-link">Benutzereinstellungen</a>
          </li>
          <li class="divider"></li>
          <li>
            <a
              href="#LastViewed"
              class="nav-link action"
              data-action="showLastViewed"
              >Zuletzt Angezeigt</a
            >
          </li>
          <li class="divider"></li>
          <li><a href="#About" class="nav-link">Über</a></li>
          <li>
            <a href="javascript:" class="nav-link action" data-action="logout"
              >Abmelden</a
            >
          </li>
        </ul>
					</li>
				</ul>
				<div
					class="logo-head_line"
					style="background: gray none repeat scroll 0% 0%; height: 5px; box-shadow: gray 0px 3px 8px 2px;"
				></div>
			</div>
		</b-sidebar>
	</div>
</template>

<script lang="ts">
import { Component, Vue } from "vue-property-decorator";

@Component
export default class Sidebar extends Vue {}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss">

.nav-menu-dropdown {
  position: relative;
  display: inline-block;
}

.dropdown-menu {
  display: none;
  position: absolute;
  background-color: #f9f9f9;
  min-width: 160px;
  box-shadow: 0px 8px 16px 0px rgba(0,0,0,0.2);
  z-index: 1;
  font-size: initial;
  margin-left: -151px;
}

.dropdown-menu a {
  color: black;
  padding: 12px 16px;
  text-decoration: none;
  display: block;
}

.dropdown-menu a:hover {background-color: #f1f1f1}

.dropdown:hover .dropdown-menu {
  display: block;
}

.dropdown:hover .dropbtn {
  background-color: #3e8e41;
}

.menu-container:hover {
  background-color: #ddd;
}

</style>
