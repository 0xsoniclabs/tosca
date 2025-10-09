include_guard(GLOBAL)


option(TOSCA_ASSERT "Enable Tosca specific assertions.")
add_compile_options($<$<BOOL:${TOSCA_ASSERT}>:TOSCA_ASSERT_ENABLED>)