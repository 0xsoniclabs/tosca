include_guard(GLOBAL)

option(TOSCA_ASAN "Enable AddressSanitizer for Tosca targets.")
if(TOSCA_ASAN)
  add_compile_options(
    $<$<CXX_COMPILER_ID:Clang>:-fno-omit-frame-pointer>
    $<$<CXX_COMPILER_ID:Clang>:-fsanitize=address>
    )
  add_link_options(
    $<$<CXX_COMPILER_ID:Clang>:-fno-omit-frame-pointer>
    $<$<CXX_COMPILER_ID:Clang>:-fsanitize=address>
    )
endif()