// SPDX-License-Identifier: GPL-3.0-or-later

export function renderDockContent(content, model, createButton, createDivider) {
  content.destroy_all_children();
  for (const item of model) {
    content.add_child(item.divider ? createDivider() : createButton(item));
  }
}
