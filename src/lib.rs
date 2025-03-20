pub mod reflection;

mod impls;
#[cfg(test)]
mod tests;
mod utils;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::{null, null_mut};
use std::{mem, ptr};

use impls::{FileSystemImpl, OwnedBlobImpl, StaticBlobImpl};

use crate::sys::{Interface, vtable_call};

pub use utils::{Error, Result};

pub mod sys {
	pub use slang_sys::*;
}

pub use sys::{
	SlangBindingType as BindingType, SlangCompileTarget as CompileTarget,
	SlangDebugInfoLevel as DebugInfoLevel, SlangFloatingPointMode as FloatingPointMode,
	SlangImageFormat as ImageFormat, SlangLineDirectiveMode as LineDirectiveMode,
	SlangMatrixLayoutMode as MatrixLayoutMode, SlangOptimizationLevel as OptimizationLevel,
	SlangParameterCategory as ParameterCategory, SlangResourceAccess as ResourceAccess,
	SlangResourceShape as ResourceShape, SlangScalarType as ScalarType,
	SlangSourceLanguage as SourceLanguage, SlangStage as Stage, SlangTypeKind as TypeKind,
	SlangUUID as UUID, slang_CompilerOptionName as CompilerOptionName,
};
use utils::define_interface;

pub(crate) fn succeeded(result: sys::SlangResult) -> bool {
	result >= 0
}

pub struct ProfileID(sys::SlangProfileID);

impl ProfileID {
	pub const UNKNOWN: ProfileID = ProfileID(sys::SlangProfileID_SlangProfileUnknown);
}

pub struct CapabilityID(sys::SlangCapabilityID);

impl CapabilityID {
	pub const UNKNOWN: CapabilityID = CapabilityID(sys::SlangCapabilityID_SlangCapabilityUnknown);
}

define_interface!(Blob, sys::slang_IBlob, Debug);

impl Blob {
	pub fn as_slice(&self) -> &[u8] {
		let ptr = unsafe { vtable_call!(self.0, getBufferPointer()) };
		let size = unsafe { vtable_call!(self.0, getBufferSize()) };
		unsafe { std::slice::from_raw_parts(ptr as *const u8, size) }
	}

	pub fn as_str(&self) -> std::result::Result<&str, std::str::Utf8Error> {
		std::str::from_utf8(self.as_slice())
	}
}

impl From<&'static [u8]> for Blob {
	#[inline]
	fn from(value: &'static [u8]) -> Self {
		let blob = Box::leak(Box::new(StaticBlobImpl::new(value)));

		Blob(blob as *mut _ as *mut _)
	}
}

impl From<&'static str> for Blob {
	#[inline]
	fn from(value: &'static str) -> Self {
		Self::from(value.as_bytes())
	}
}

impl From<Vec<u8>> for Blob {
	#[inline]
	fn from(value: Vec<u8>) -> Self {
		let blob = Box::leak(Box::new(OwnedBlobImpl::new(value)));

		Blob(blob as *mut _ as *mut _)
	}
}

impl From<String> for Blob {
	#[inline]
	fn from(value: String) -> Self {
		Self::from(value.into_bytes())
	}
}

define_interface!(GlobalSession, sys::slang_IGlobalSession, Debug);

impl GlobalSession {
	pub fn new() -> utils::Result<Self> {
		let mut session = ptr::null_mut();
		utils::result_from_ffi(unsafe {
			sys::slang_createGlobalSession(sys::SLANG_API_VERSION as _, &mut session)
		})?;

		Ok(Self(session))
	}

	pub fn new_without_core_module() -> utils::Result<Self> {
		let mut session = null_mut();
		utils::result_from_ffi(unsafe {
			sys::slang_createGlobalSessionWithoutCoreModule(
				sys::SLANG_API_VERSION as _,
				&mut session,
			)
		})?;

		Ok(Self(session))
	}

	pub fn create_session(&self, desc: &SessionDesc) -> utils::Result<Session> {
		let mut session = null_mut();
		utils::result_from_ffi(unsafe {
			vtable_call!(
				self.0,
				createSession(desc as *const SessionDesc as *const _, &mut session)
			)
		})?;
		Ok(Session(session))
	}

	pub fn find_profile(&self, name: &str) -> ProfileID {
		let name = CString::new(name).unwrap();
		ProfileID(unsafe { vtable_call!(self.0, findProfile(name.as_ptr())) })
	}

	pub fn find_capability(&self, name: &str) -> CapabilityID {
		let name = CString::new(name).unwrap();
		CapabilityID(unsafe { vtable_call!(self.0, findCapability(name.as_ptr())) })
	}
}

define_interface!(Session, sys::slang_ISession, Debug);

impl Session {
	pub fn load_module(&self, name: &str) -> utils::Result<Module> {
		let name = CString::new(name).unwrap();
		let mut diagnostics = null_mut();

		let module = unsafe { vtable_call!(self.0, loadModule(name.as_ptr(), &mut diagnostics)) };

		if module.is_null() {
			Err(Error::Blob(Blob(diagnostics)))
		} else {
			Ok(Module(module))
		}
	}

	pub fn create_composite_component_type(
		&self,
		component_types: &[ComponentType],
	) -> utils::Result<ComponentType> {
		let mut composite_component_type = ptr::null_mut();
		let mut diagnostics = ptr::null_mut();

		utils::result_from_blob(
			unsafe {
				vtable_call!(
					self.0,
					createCompositeComponentType(
						component_types.as_ptr().cast(),
						component_types.len() as _,
						&mut composite_component_type,
						&mut diagnostics
					)
				)
			},
			diagnostics,
		)?;
		Ok(ComponentType(composite_component_type))
	}
}

define_interface!(Metadata, sys::slang_IMetadata, Debug);

impl Metadata {
	pub fn is_parameter_location_used(
		&self,
		category: ParameterCategory,
		space_index: u64,
		register_index: u64,
	) -> Option<bool> {
		let mut used = false;
		let res = unsafe {
			vtable_call!(
				self.0,
				isParameterLocationUsed(category, space_index, register_index, &mut used)
			)
		};
		succeeded(res).then_some(used)
	}
}

define_interface!(ProgramLayout, sys::slang_ProgramLayout, Debug);

define_interface!(ComponentType, sys::slang_IComponentType, Debug);

impl ComponentType {
	pub fn layout(&self, target_index: i64) -> Result<ProgramLayout> {
		let mut diagnostics = ptr::null_mut();

		let program_layout =
			unsafe { vtable_call!(self.0, getLayout(target_index, &mut diagnostics)) };

		if program_layout.is_null() {
			Err(Error::Blob(Blob(diagnostics)))
		} else {
			Ok(ProgramLayout(program_layout))
		}
	}

	pub fn link(&self) -> utils::Result<ComponentType> {
		let mut linked_component_type = null_mut();
		let mut diagnostics = null_mut();

		utils::result_from_blob(
			unsafe { vtable_call!(self.0, link(&mut linked_component_type, &mut diagnostics)) },
			diagnostics,
		)?;

		Ok(ComponentType(linked_component_type))
	}

	pub fn target_code(&self, target: i64) -> utils::Result<Blob> {
		let mut code = null_mut();
		let mut diagnostics = null_mut();

		utils::result_from_blob(
			unsafe { vtable_call!(self.0, getTargetCode(target, &mut code, &mut diagnostics)) },
			diagnostics,
		)?;

		Ok(Blob(code))
	}

	pub fn entry_point_code(&self, index: i64, target: i64) -> utils::Result<Blob> {
		let mut code = null_mut();
		let mut diagnostics = null_mut();

		utils::result_from_blob(
			unsafe {
				vtable_call!(
					self.0,
					getEntryPointCode(index, target, &mut code, &mut diagnostics)
				)
			},
			diagnostics,
		)?;

		Ok(Blob(code))
	}

	pub fn target_metadata(&self, target_index: i64) -> utils::Result<Metadata> {
		let mut metadata = null_mut();
		let mut diagnostics = null_mut();

		utils::result_from_blob(
			unsafe {
				vtable_call!(
					self.0,
					getTargetMetadata(target_index, &mut metadata, &mut diagnostics)
				)
			},
			diagnostics,
		)?;

		Ok(Metadata(metadata))
	}

	pub fn entry_point_metadata(
		&self,
		entry_point_index: i64,
		target_index: i64,
	) -> utils::Result<Metadata> {
		let mut metadata = null_mut();
		let mut diagnostics = null_mut();

		utils::result_from_blob(
			unsafe {
				vtable_call!(
					self.0,
					getEntryPointMetadata(
						entry_point_index,
						target_index,
						&mut metadata,
						&mut diagnostics
					)
				)
			},
			diagnostics,
		)?;

		Ok(Metadata(metadata))
	}
}

define_interface!(EntryPoint, sys::slang_IEntryPoint, ComponentType);

impl EntryPoint {
	pub fn function_reflection(&self) -> &reflection::Function {
		let ptr = unsafe { vtable_call!(self.0, getFunctionReflection()) };
		unsafe { &*(ptr as *const _) }
	}
}

define_interface!(TypeConformance, sys::slang_ITypeConformance, ComponentType);

define_interface!(Module, sys::slang_IModule, ComponentType);

impl Module {
	pub fn find_entry_point_by_name(&self, name: &str) -> utils::Result<EntryPoint> {
		let name = CString::new(name).unwrap();
		let mut entry_point = null_mut();
		utils::result_from_ffi(unsafe {
			vtable_call!(
				self.0,
				findEntryPointByName(name.as_ptr(), &mut entry_point)
			)
		})?;

		Ok(EntryPoint(entry_point))
	}

	pub fn entry_point_count(&self) -> u32 {
		unsafe { vtable_call!(self.0, getDefinedEntryPointCount()) as u32 }
	}

	pub fn entry_point_by_index(&self, index: u32) -> utils::Result<EntryPoint> {
		let mut entry_point = null_mut();

		utils::result_from_ffi(unsafe {
			vtable_call!(self.0, getDefinedEntryPoint(index as _, &mut entry_point))
		})?;

		Ok(EntryPoint(entry_point))
	}

	pub fn entry_points(&self) -> impl ExactSizeIterator<Item = EntryPoint> {
		(0..self.entry_point_count()).map(move |i| self.entry_point_by_index(i).unwrap())
	}

	pub fn name(&self) -> &str {
		let name = unsafe { vtable_call!(self.0, getName()) };
		unsafe { CStr::from_ptr(name).to_str().unwrap() }
	}

	pub fn file_path(&self) -> &str {
		let path = unsafe { vtable_call!(self.0, getFilePath()) };
		unsafe { CStr::from_ptr(path).to_str().unwrap() }
	}

	pub fn unique_identity(&self) -> &str {
		let identity = unsafe { vtable_call!(self.0, getUniqueIdentity()) };
		unsafe { CStr::from_ptr(identity).to_str().unwrap() }
	}

	pub fn dependency_file_count(&self) -> i32 {
		unsafe { vtable_call!(self.0, getDependencyFileCount()) }
	}

	pub fn dependency_file_path(&self, index: i32) -> &str {
		let path = unsafe { vtable_call!(self.0, getDependencyFilePath(index)) };
		unsafe { CStr::from_ptr(path).to_str().unwrap() }
	}

	pub fn dependency_file_paths(&self) -> impl ExactSizeIterator<Item = &str> {
		(0..self.dependency_file_count()).map(move |i| self.dependency_file_path(i))
	}

	pub fn module_reflection(&self) -> &reflection::Decl {
		let ptr = unsafe { vtable_call!(self.0, getModuleReflection()) };
		unsafe { &*(ptr as *const _) }
	}
}

#[repr(transparent)]
pub struct TargetDesc<'a> {
	inner: sys::slang_TargetDesc,
	_phantom: PhantomData<&'a ()>,
}

impl std::ops::Deref for TargetDesc<'_> {
	type Target = sys::slang_TargetDesc;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl Default for TargetDesc<'_> {
	fn default() -> Self {
		Self {
			inner: sys::slang_TargetDesc {
				structureSize: std::mem::size_of::<sys::slang_TargetDesc>(),
				..unsafe { std::mem::zeroed() }
			},
			_phantom: PhantomData,
		}
	}
}

impl<'a> TargetDesc<'a> {
	pub fn format(mut self, format: CompileTarget) -> Self {
		self.inner.format = format;
		self
	}

	pub fn profile(mut self, profile: ProfileID) -> Self {
		self.inner.profile = profile.0;
		self
	}

	pub fn options(mut self, options: &'a CompilerOptions) -> Self {
		self.inner.compilerOptionEntries = options.options.as_ptr() as _;
		self.inner.compilerOptionEntryCount = options.options.len() as _;
		self
	}
}

pub trait FileSystem {
	fn load_file(&self, path: &str) -> utils::Result<Blob>;
}

#[repr(transparent)]
pub struct SessionDesc<'a> {
	inner: sys::slang_SessionDesc,
	_phantom: PhantomData<&'a ()>,
}

impl std::ops::Deref for SessionDesc<'_> {
	type Target = sys::slang_SessionDesc;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl Default for SessionDesc<'_> {
	fn default() -> Self {
		Self {
			inner: sys::slang_SessionDesc {
				structureSize: std::mem::size_of::<sys::slang_SessionDesc>(),
				..unsafe { std::mem::zeroed() }
			},
			_phantom: PhantomData,
		}
	}
}

impl<'a> SessionDesc<'a> {
	pub fn targets(mut self, targets: &'a [TargetDesc]) -> Self {
		self.inner.targets = targets.as_ptr() as _;
		self.inner.targetCount = targets.len() as _;
		self
	}

	pub fn search_paths(mut self, paths: &'a [*const i8]) -> Self {
		self.inner.searchPaths = paths.as_ptr();
		self.inner.searchPathCount = paths.len() as _;
		self
	}

	pub fn options(mut self, options: &'a CompilerOptions) -> Self {
		self.inner.compilerOptionEntries = options.options.as_ptr() as _;
		self.inner.compilerOptionEntryCount = options.options.len() as _;
		self
	}

	pub fn file_system(mut self, file_system: impl FileSystem + 'static) -> Self {
		let file_system = Box::leak(Box::new(FileSystemImpl::new(Box::new(file_system))));
		self.inner.fileSystem = file_system as *mut _ as *mut _;
		self
	}
}

macro_rules! option {
	($name:ident, $func:ident($p_name:ident: $p_type:ident)) => {
		#[inline(always)]
		pub fn $func(self, $p_name: $p_type) -> Self {
			self.push_ints(CompilerOptionName::$name, $p_name as _, 0)
		}
	};

	($name:ident, $func:ident($p_name:ident: &str)) => {
		#[inline(always)]
		pub fn $func(self, $p_name: &str) -> Self {
			self.push_str1(CompilerOptionName::$name, $p_name)
		}
	};

	($name:ident, $func:ident($p_name1:ident: &str, $p_name2:ident: &str)) => {
		#[inline(always)]
		pub fn $func(self, $p_name1: &str, $p_name2: &str) -> Self {
			self.push_str2(CompilerOptionName::$name, $p_name1, $p_name2)
		}
	};
}

#[derive(Default)]
pub struct CompilerOptions {
	strings: Vec<CString>,
	options: Vec<sys::slang_CompilerOptionEntry>,
}

impl CompilerOptions {
	fn push_ints(mut self, name: CompilerOptionName, i0: i32, i1: i32) -> Self {
		self.options.push(sys::slang_CompilerOptionEntry {
			name,
			value: sys::slang_CompilerOptionValue {
				kind: sys::slang_CompilerOptionValueKind::Int,
				intValue0: i0,
				intValue1: i1,
				stringValue0: null(),
				stringValue1: null(),
			},
		});

		self
	}

	fn push_strings(mut self, name: CompilerOptionName, s0: *const i8, s1: *const i8) -> Self {
		self.options.push(sys::slang_CompilerOptionEntry {
			name,
			value: sys::slang_CompilerOptionValue {
				kind: sys::slang_CompilerOptionValueKind::String,
				intValue0: 0,
				intValue1: 0,
				stringValue0: s0,
				stringValue1: s1,
			},
		});

		self
	}

	fn push_str1(mut self, name: CompilerOptionName, s0: &str) -> Self {
		let s0 = CString::new(s0).unwrap();
		let s0_ptr = s0.as_ptr();
		self.strings.push(s0);

		self.push_strings(name, s0_ptr, null())
	}

	fn push_str2(mut self, name: CompilerOptionName, s0: &str, s1: &str) -> Self {
		let s0 = CString::new(s0).unwrap();
		let s0_ptr = s0.as_ptr();
		self.strings.push(s0);

		let s1 = CString::new(s1).unwrap();
		let s1_ptr = s1.as_ptr();
		self.strings.push(s1);

		self.push_strings(name, s0_ptr, s1_ptr)
	}
}

impl CompilerOptions {
	option!(MacroDefine, macro_define(key: &str, value: &str));
	option!(Include, include(path: &str));
	option!(Language, language(language: SourceLanguage));
	option!(MatrixLayoutColumn, matrix_layout_column(enable: bool));
	option!(MatrixLayoutRow, matrix_layout_row(enable: bool));

	#[inline(always)]
	pub fn profile(self, profile: ProfileID) -> Self {
		self.push_ints(CompilerOptionName::Profile, profile.0 as _, 0)
	}

	option!(Stage, stage(stage: Stage));
	option!(Target, target(target: CompileTarget));
	option!(WarningsAsErrors, warnings_as_errors(warning_codes: &str));
	option!(DisableWarnings, disable_warnings(warning_codes: &str));
	option!(EnableWarning, enable_warning(warning_code: &str));
	option!(DisableWarning, disable_warning(warning_code: &str));
	option!(ReportDownstreamTime, report_downstream_time(enable: bool));
	option!(ReportPerfBenchmark, report_perf_benchmark(enable: bool));
	option!(SkipSPIRVValidation, skip_spirv_validation(enable: bool));

	// Target
	#[inline(always)]
	pub fn capability(self, capability: CapabilityID) -> Self {
		self.push_ints(CompilerOptionName::Capability, capability.0 as _, 0)
	}

	option!(DefaultImageFormatUnknown, default_image_format_unknown(enable: bool));
	option!(DisableDynamicDispatch, disable_dynamic_dispatch(enable: bool));
	option!(DisableSpecialization, disable_specialization(enable: bool));
	option!(FloatingPointMode, floating_point_mode(mode: FloatingPointMode));
	option!(DebugInformation, debug_information(level: DebugInfoLevel));
	option!(LineDirectiveMode, line_directive_mode(mode: LineDirectiveMode));
	option!(Optimization, optimization(level: OptimizationLevel));
	option!(Obfuscate, obfuscate(enable: bool));
	option!(VulkanUseEntryPointName, vulkan_use_entry_point_name(enable: bool));
	option!(GLSLForceScalarLayout, glsl_force_scalar_layout(enable: bool));
	option!(EmitSpirvDirectly, emit_spirv_directly(enable: bool));

	// Debugging
	option!(NoCodeGen, no_code_gen(enable: bool));

	// Experimental
	option!(NoMangle, no_mangle(enable: bool));
	option!(ValidateUniformity, validate_uniformity(enable: bool));
}
