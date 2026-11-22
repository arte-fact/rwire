### Changed: `Sidebar` header padding and footer text like the old front

- The header slot is padded 14 px 14 px 10 px, the top safe area included
  (`St::PxSidebarHead`, `St::PtSidebarHeadSafe`, `St::PbSidebarHead`).
- The footer slot sets its text to 12.5 px (`St::Text12_5`, a new px size);
  links elsewhere keep their size.
