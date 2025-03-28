use crate::ProgramLayout;

use super::{
	EntryPoint, Function, Type, TypeLayout, TypeParameter, Variable, VariableLayout, rcall,
};
use slang_sys as sys;

impl ProgramLayout {
	pub fn parameter_count(&self) -> u32 {
		rcall!(spReflection_GetParameterCount(self.0))
	}

	pub fn parameter_by_index(&self, index: u32) -> Option<&VariableLayout> {
		rcall!(spReflection_GetParameterByIndex(self.0, index) as Option<&VariableLayout>)
	}

	pub fn parameters(&self) -> impl ExactSizeIterator<Item = &VariableLayout> {
		(0..self.parameter_count())
			.map(move |i| rcall!(spReflection_GetParameterByIndex(self.0, i) as &VariableLayout))
	}

	pub fn type_parameter_count(&self) -> u32 {
		rcall!(spReflection_GetTypeParameterCount(self.0))
	}

	pub fn type_parameter_by_index(&self, index: u32) -> Option<&TypeParameter> {
		rcall!(spReflection_GetTypeParameterByIndex(self.0, index) as Option<&TypeParameter>)
	}

	pub fn type_parameters(&self) -> impl ExactSizeIterator<Item = &TypeParameter> {
		(0..self.type_parameter_count())
			.map(move |i| rcall!(spReflection_GetTypeParameterByIndex(self.0, i) as &TypeParameter))
	}

	pub fn find_type_parameter_by_name(&self, name: &str) -> Option<&TypeParameter> {
		let name = std::ffi::CString::new(name).unwrap();
		rcall!(spReflection_FindTypeParameter(self.0, name.as_ptr()) as Option<&TypeParameter>)
	}

	pub fn entry_point_count(&self) -> u32 {
		rcall!(spReflection_getEntryPointCount(self.0)) as _
	}

	pub fn entry_point_by_index(&self, index: u32) -> Option<&EntryPoint> {
		rcall!(spReflection_getEntryPointByIndex(self.0, index as _) as Option<&EntryPoint>)
	}

	pub fn entry_points(&self) -> impl ExactSizeIterator<Item = &EntryPoint> {
		(0..self.entry_point_count())
			.map(move |i| rcall!(spReflection_getEntryPointByIndex(self.0, i as _) as &EntryPoint))
	}

	pub fn find_entry_point_by_name(&self, name: &str) -> Option<&EntryPoint> {
		let name = std::ffi::CString::new(name).unwrap();
		rcall!(spReflection_findEntryPointByName(self.0, name.as_ptr()) as Option<&EntryPoint>)
	}

	pub fn global_constant_buffer_binding(&self) -> u64 {
		rcall!(spReflection_getGlobalConstantBufferBinding(self.0))
	}

	pub fn global_constant_buffer_size(&self) -> usize {
		rcall!(spReflection_getGlobalConstantBufferSize(self.0))
	}

	pub fn find_type_by_name(&self, name: &str) -> Option<&Type> {
		let name = std::ffi::CString::new(name).unwrap();
		rcall!(spReflection_FindTypeByName(self.0, name.as_ptr()) as Option<&Type>)
	}

	pub fn find_function_by_name(&self, name: &str) -> Option<&Function> {
		let name = std::ffi::CString::new(name).unwrap();
		rcall!(spReflection_FindFunctionByName(self.0, name.as_ptr()) as Option<&Function>)
	}

	pub fn find_function_by_name_in_type(&self, ty: &Type, name: &str) -> Option<&Function> {
		let name = std::ffi::CString::new(name).unwrap();
		rcall!(spReflection_FindFunctionByNameInType(
			self.0,
			ty as *const _ as *mut _,
			name.as_ptr()
		) as Option<&Function>)
	}

	pub fn find_var_by_name_in_type(&self, ty: &Type, name: &str) -> Option<&Variable> {
		let name = std::ffi::CString::new(name).unwrap();
		rcall!(
			spReflection_FindVarByNameInType(self.0, ty as *const _ as *mut _, name.as_ptr())
				as Option<&Variable>
		)
	}

	pub fn type_layout(&self, ty: &Type, rules: sys::SlangLayoutRules) -> Option<&TypeLayout> {
		rcall!(
			spReflection_GetTypeLayout(self.0, ty as *const _ as *mut _, rules)
				as Option<&TypeLayout>
		)
	}

	// TODO: specialize_type
	// TODO: specialize_generic
	// TODO: is_sub_type

	pub fn hashed_string_count(&self) -> u64 {
		rcall!(spReflection_getHashedStringCount(self.0))
	}

	pub fn hashed_string(&self, index: u64) -> Option<&str> {
		let mut len = 0;
		let result = rcall!(spReflection_getHashedString(self.0, index, &mut len));

		(!result.is_null()).then(|| {
			let slice = unsafe { std::slice::from_raw_parts(result as *const u8, len) };
			std::str::from_utf8(slice).unwrap()
		})
	}

	pub fn global_params_type_layout(&self) -> &TypeLayout {
		rcall!(spReflection_getGlobalParamsTypeLayout(self.0) as &TypeLayout)
	}

	pub fn global_params_var_layout(&self) -> &VariableLayout {
		rcall!(spReflection_getGlobalParamsVarLayout(self.0) as &VariableLayout)
	}
}

pub fn compute_string_hash(string: &str) -> u32 {
	rcall!(spComputeStringHash(string, string.len()))
}
