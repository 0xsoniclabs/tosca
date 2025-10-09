
###########################################################
# Library
add_library(evmzero SHARED
  evmzero.cc
  allocator.cc
  interpreter.cc
  memory.cc
  opcodes.cc
  stack.cc
)

get_target_property(EVMZERO_COMPILE_DEFINITIONS evmzero COMPILE_DEFINITIONS)
get_target_property(EVMZERO_COMPILE_OPTIONS evmzero COMPILE_OPTIONS)
# The target properties contain generator expressions which cannot be resolved through
# configure_file. The workaround is to use file(GENERATE ... TARGET ...) to expand
# the generator expressions.
file(GENERATE OUTPUT target_properties.inc CONTENT
  "constexpr auto kCompileDefinitions = \"${EVMZERO_COMPILE_DEFINITIONS}\";
  constexpr auto kCompileOptions = \"${EVMZERO_COMPILE_OPTIONS}\";"
  TARGET evmzero
)
configure_file(build_info.h.in build_info.h)
target_include_directories(evmzero PUBLIC ${CMAKE_CURRENT_BINARY_DIR})

target_link_libraries(evmzero PUBLIC tosca_common ethash::keccak evmc::evmc intx::intx)

option(EVMZERO_MIMALLOC "Enable mimalloc for evmzero" ON)
if(EVMZERO_MIMALLOC)
  target_compile_definitions(evmzero PRIVATE EVMZERO_MIMALLOC_ENABLED)
  target_link_libraries(evmzero PRIVATE mimalloc-obj)
endif()

option(EVMZERO_TRACY "Enable Tracy profiler for evmzero" OFF)
target_compile_definitions(evmzero PRIVATE $<$<BOOL:${EVMZERO_TRACY}>:EVMZERO_TRACY_ENABLED>)
target_link_libraries(evmzero PUBLIC $<$<BOOL:${EVMZERO_TRACY}>:Tracy::TracyClient>)

###########################################################
# Tests
include(GoogleTest)

file(GLOB_RECURSE evmzero_tests CONFIGURE_DEPENDS *_test.cc)
foreach(evmzero_test ${evmzero_tests})
  get_filename_component(evmzero_test_target ${evmzero_test} NAME_WE)

  add_executable(${evmzero_test_target} ${evmzero_test})
  target_link_libraries(${evmzero_test_target} evmzero GTest::gmock GTest::gtest_main)
  gtest_discover_tests(${evmzero_test_target})
endforeach()

###########################################################
# Benchmarks

file(GLOB_RECURSE evmzero_benchmarks CONFIGURE_DEPENDS *_benchmark.cc)
foreach(evmzero_benchmark ${evmzero_benchmarks})
  get_filename_component(evmzero_benchmark_target ${evmzero_benchmark} NAME_WE)

  add_executable(${evmzero_benchmark_target} ${evmzero_benchmark})
  target_link_libraries(${evmzero_benchmark_target} evmzero benchmark::benchmark benchmark::benchmark_main)

  # Always include the CPU profiler in benchmarks.
  if (NOT ${CMAKE_SYSTEM_NAME} MATCHES "Darwin")
    target_link_libraries(${evmzero_benchmark_target} -Wl,--no-as-needed profiler  -Wl,--as-needed)
  endif()
endforeach()
